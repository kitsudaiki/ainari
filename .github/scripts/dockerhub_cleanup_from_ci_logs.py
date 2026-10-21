#!/usr/bin/env python3
"""
One-time cleanup of untagged images on Docker Hub, which were pushed by the build-and-test workflow.

Docker Hub provides no API to list untagged manifests, so the digests are collected from the logs
of the old workflow-runs instead:

1. the logs of all runs of the workflow are downloaded via the GitHub API
2. every log-file, which mentions exactly one of the given repositories, belongs to a job of this
   repository, so all sha256-digests in this file are candidates for this repository
3. candidates, which are no manifest in the repository (layers, configs, base-images, already
   deleted manifests), are dropped via a HEAD-request, which doesn't count to the pull-limit
4. all manifests, which are reachable from a currently existing tag, are kept
5. the remaining manifests are deleted, manifest-lists first, so their children are unreferenced
   when they are deleted afterwards

Only manifests, which are still visible in the retained logs (default 90 days), can be found.

Environment:
    GITHUB_TOKEN, GITHUB_REPOSITORY     access to the workflow-runs and their logs
    DOCKERHUB_USERNAME, DOCKERHUB_TOKEN token with Read, Write & Delete permission
"""

import argparse
import base64
import io
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
import zipfile

GITHUB_API = "https://api.github.com"
REGISTRY = "https://registry-1.docker.io/v2"
AUTH_URL = "https://auth.docker.io/token"

INDEX_TYPES = {
    "application/vnd.oci.image.index.v1+json",
    "application/vnd.docker.distribution.manifest.list.v2+json",
}
MANIFEST_TYPES = INDEX_TYPES | {
    "application/vnd.oci.image.manifest.v1+json",
    "application/vnd.docker.distribution.manifest.v2+json",
}
ACCEPT = ",".join(sorted(MANIFEST_TYPES))

DIGEST_RE = re.compile(r"sha256:[0-9a-f]{64}")


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def request(url, method="GET", headers=None, follow_redirects=True, retries=5):
    """Send a request and return (status, headers, body). Retries on rate-limits and 5xx."""
    opener = urllib.request.build_opener() if follow_redirects else urllib.request.build_opener(NoRedirect)
    for attempt in range(retries):
        req = urllib.request.Request(url, method=method, headers=headers or {})
        try:
            with opener.open(req, timeout=120) as resp:
                return resp.status, resp.headers, resp.read()
        except urllib.error.HTTPError as e:
            if e.code in (429, 500, 502, 503, 504) and attempt + 1 < retries:
                delay = int(e.headers.get("Retry-After", 0) or 0) or 2 ** (attempt + 1)
                print(f"  HTTP {e.code} for {method} {url}, retry in {delay}s", file=sys.stderr)
                time.sleep(delay)
                continue
            return e.code, e.headers, e.read()
        except urllib.error.URLError as e:
            if attempt + 1 < retries:
                time.sleep(2 ** (attempt + 1))
                continue
            raise RuntimeError(f"{method} {url} failed: {e}") from e
    raise RuntimeError(f"{method} {url} failed after {retries} attempts")


# ---------------------------------------------------------------------------------------------
# GitHub
# ---------------------------------------------------------------------------------------------

def github_headers():
    return {
        "Authorization": f"Bearer {os.environ['GITHUB_TOKEN']}",
        "Accept": "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
    }


def list_runs(repository, workflow):
    page = 1
    while True:
        url = f"{GITHUB_API}/repos/{repository}/actions/workflows/{workflow}/runs?per_page=100&page={page}"
        status, _, body = request(url, headers=github_headers())
        if status != 200:
            raise RuntimeError(f"failed to list workflow-runs (HTTP {status}): {body[:500]!r}")
        runs = json.loads(body)["workflow_runs"]
        if not runs:
            return
        yield from runs
        page += 1


def download_logs(repository, run_id):
    """Return the zip-archive of the logs of a run or None, if the logs are expired."""
    url = f"{GITHUB_API}/repos/{repository}/actions/runs/{run_id}/logs"
    # the redirect points to a pre-signed blob-url, which must not get the GitHub-token
    status, headers, body = request(url, headers=github_headers(), follow_redirects=False)
    if status in (301, 302, 307, 308):
        status, _, body = request(headers["Location"])
    if status != 200:
        return None
    return zipfile.ZipFile(io.BytesIO(body))


def collect_candidates(zip_file, repositories, candidates):
    """Add the digests of all log-files, which belong to exactly one repository."""
    repo_patterns = {
        repo: re.compile(r"(?<![\w.-])" + re.escape(repo) + r"(?![\w.-]|/)") for repo in repositories
    }
    for name in zip_file.namelist():
        if not name.endswith(".txt"):
            continue
        text = zip_file.read(name).decode("utf-8", errors="replace")
        mentioned = [repo for repo, pattern in repo_patterns.items() if pattern.search(text)]
        if len(mentioned) != 1:
            continue
        candidates[mentioned[0]].update(DIGEST_RE.findall(text))


# ---------------------------------------------------------------------------------------------
# Docker Hub
# ---------------------------------------------------------------------------------------------

class Registry:
    def __init__(self, username, token):
        self.basic = base64.b64encode(f"{username}:{token}".encode()).decode()
        self.tokens = {}

    def token(self, repository, force=False):
        cached = self.tokens.get(repository)
        # the bearer-tokens of Docker Hub expire after 300 seconds
        if cached and not force and time.time() - cached[1] < 240:
            return cached[0]
        url = f"{AUTH_URL}?service=registry.docker.io&scope=repository:{repository}:pull,push,delete"
        status, _, body = request(url, headers={"Authorization": f"Basic {self.basic}"})
        if status != 200:
            raise RuntimeError(f"failed to get token for {repository} (HTTP {status}): {body[:500]!r}")
        token = json.loads(body)["token"]
        self.tokens[repository] = (token, time.time())
        return token

    def call(self, repository, path, method="GET", accept=ACCEPT):
        url = f"{REGISTRY}/{repository}/{path}"
        for force in (False, True):
            headers = {"Authorization": f"Bearer {self.token(repository, force)}", "Accept": accept}
            status, resp_headers, body = request(url, method=method, headers=headers)
            if status != 401:
                break
        return status, resp_headers, body

    def tags(self, repository):
        status, _, body = self.call(repository, "tags/list?n=10000", accept="application/json")
        if status == 404:
            return []
        if status != 200:
            raise RuntimeError(f"failed to list tags of {repository} (HTTP {status}): {body[:500]!r}")
        return json.loads(body).get("tags") or []

    def media_type(self, repository, digest):
        """HEAD-request, which doesn't count to the pull-limit. Returns None, if not a manifest."""
        status, headers, _ = self.call(repository, f"manifests/{digest}", method="HEAD")
        if status != 200:
            return None
        media_type = (headers.get("Content-Type") or "").split(";")[0].strip()
        return media_type if media_type in MANIFEST_TYPES else None

    def reachable(self, repository, reference, result):
        """Add the digest of the reference and of all manifests it references."""
        # HEAD first, because only manifest-lists have to be read via GET, which counts as pull
        status, headers, _ = self.call(repository, f"manifests/{reference}", method="HEAD")
        if status != 200:
            raise RuntimeError(f"failed to get {repository}:{reference} (HTTP {status})")
        digest = headers.get("Docker-Content-Digest")
        if not digest:
            raise RuntimeError(f"no digest for {repository}:{reference}")
        result.add(digest)
        if (headers.get("Content-Type") or "").split(";")[0].strip() not in INDEX_TYPES:
            return
        status, _, body = self.call(repository, f"manifests/{digest}")
        if status != 200:
            raise RuntimeError(f"failed to get {repository}@{digest} (HTTP {status}): {body[:500]!r}")
        for child in json.loads(body).get("manifests", []):
            if child["digest"] not in result:
                self.reachable(repository, child["digest"], result)

    def delete(self, repository, digest):
        status, _, body = self.call(repository, f"manifests/{digest}", method="DELETE")
        return status, body


# ---------------------------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--workflow", default="build_test.yml", help="workflow-file, which pushed the images")
    parser.add_argument("--repositories", required=True, help="comma-separated list of Docker Hub repositories")
    parser.add_argument("--delete", action="store_true", help="really delete, otherwise only a dry-run")
    args = parser.parse_args()

    github_repository = os.environ["GITHUB_REPOSITORY"]
    repositories = [r.strip() for r in args.repositories.split(",") if r.strip()]
    registry = Registry(os.environ["DOCKERHUB_USERNAME"], os.environ["DOCKERHUB_TOKEN"])

    # collect candidates from the logs
    candidates = {repo: set() for repo in repositories}
    runs = expired = 0
    for run in list_runs(github_repository, args.workflow):
        if run.get("status") != "completed":
            continue
        runs += 1
        logs = download_logs(github_repository, run["id"])
        if logs is None:
            expired += 1
            continue
        collect_candidates(logs, repositories, candidates)
    print(f"checked {runs} runs, logs of {expired} runs are not available anymore")

    total_deleted = total_failed = 0
    summary = ["| repository | tags | untagged found | deleted | failed |", "|---|---|---|---|---|"]
    for repo in repositories:
        print(f"\n=== {repo}: {len(candidates[repo])} digests in the logs")

        # everything reachable from an existing tag has to be kept
        keep = set()
        tags = registry.tags(repo)
        for tag in tags:
            registry.reachable(repo, tag, keep)
        print(f"  {len(tags)} tags reference {len(keep)} manifests, which are kept")

        indexes, manifests = [], []
        for digest in sorted(candidates[repo] - keep):
            media_type = registry.media_type(repo, digest)
            if media_type in INDEX_TYPES:
                indexes.append(digest)
            elif media_type:
                manifests.append(digest)
        print(f"  {len(indexes)} untagged manifest-lists and {len(manifests)} untagged manifests found")

        deleted = failed = 0
        for digest in indexes + manifests:
            if not args.delete:
                print(f"  [dry-run] would delete {repo}@{digest}")
                continue
            status, body = registry.delete(repo, digest)
            if status in (200, 202):
                deleted += 1
                print(f"  deleted {repo}@{digest}")
            elif status == 404:
                print(f"  already gone {repo}@{digest}")
            else:
                failed += 1
                print(f"::warning::could not delete {repo}@{digest} (HTTP {status}): {body[:200]!r}")

        total_deleted += deleted
        total_failed += failed
        summary.append(f"| {repo} | {len(tags)} | {len(indexes) + len(manifests)} | {deleted} | {failed} |")

    mode = "deleted" if args.delete else "dry-run, nothing deleted"
    print(f"\n{total_deleted} manifests {mode}, {total_failed} failed")
    if "GITHUB_STEP_SUMMARY" in os.environ:
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as f:
            f.write(f"## Cleanup of untagged images ({mode})\n\n" + "\n".join(summary) + "\n")
    return 1 if total_failed else 0


if __name__ == "__main__":
    sys.exit(main())
