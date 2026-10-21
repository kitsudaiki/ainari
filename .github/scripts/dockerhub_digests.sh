#!/usr/bin/env bash
#
# Helper for cleaning up untagged images on Docker Hub, which remain after a tag was overwritten.
#
# Docker Hub provides no API to list untagged manifests, so the digests behind a tag are read
# before the tag is overwritten and deleted after the new image was pushed.
#
# Usage:
#   dockerhub_digests.sh list <repository> <tag>
#       prints the digest of the manifest behind the tag and, for a manifest-list, the digests of
#       all referenced manifests. Prints nothing, if the tag doesn't exist.
#   dockerhub_digests.sh delete <repository> <digest>...
#       deletes the manifests in the given order. Manifests, which are still referenced by a tag
#       or another manifest, are rejected by Docker Hub and only reported as warning.
#
# Requires DOCKERHUB_USERNAME and DOCKERHUB_TOKEN, where the token needs the delete-permission.

set -euo pipefail

REGISTRY="https://registry-1.docker.io"
ACCEPT="application/vnd.oci.image.index.v1+json,application/vnd.docker.distribution.manifest.list.v2+json,application/vnd.oci.image.manifest.v1+json,application/vnd.docker.distribution.manifest.v2+json"

get_token() {
    local repository="$1"
    curl -fsS -u "${DOCKERHUB_USERNAME}:${DOCKERHUB_TOKEN}" \
        "https://auth.docker.io/token?service=registry.docker.io&scope=repository:${repository}:pull,push,delete" \
        | jq -r .token
}

list_digests() {
    local repository="$1"
    local tag="$2"
    local token headers body status

    token=$(get_token "${repository}")
    headers=$(mktemp)
    body=$(mktemp)
    status=$(curl -sS -o "${body}" -D "${headers}" -w '%{http_code}' \
        -H "Authorization: Bearer ${token}" \
        -H "Accept: ${ACCEPT}" \
        "${REGISTRY}/v2/${repository}/manifests/${tag}")

    if [[ "${status}" == "404" ]]; then
        return 0
    fi
    if [[ "${status}" != "200" ]]; then
        echo "failed to get manifest of ${repository}:${tag} (HTTP ${status})" >&2
        cat "${body}" >&2
        return 1
    fi

    # the manifest-list itself first, so it is deleted before the manifests it references
    grep -i '^docker-content-digest:' "${headers}" | awk '{print $2}' | tr -d '\r'
    jq -r '.manifests[]?.digest' "${body}"
    rm -f "${headers}" "${body}"
}

delete_digests() {
    local repository="$1"
    shift
    local token digest status

    token=$(get_token "${repository}")
    for digest in "$@"; do
        status=$(curl -sS -o /dev/null -w '%{http_code}' -X DELETE \
            -H "Authorization: Bearer ${token}" \
            "${REGISTRY}/v2/${repository}/manifests/${digest}")
        case "${status}" in
            200|202) echo "deleted ${repository}@${digest}" ;;
            404)     echo "already gone ${repository}@${digest}" ;;
            *)       echo "::warning::could not delete ${repository}@${digest} (HTTP ${status})" ;;
        esac
    done
}

case "${1:-}" in
    list)   list_digests "$2" "$3" ;;
    delete) delete_digests "${@:2}" ;;
    *)      echo "usage: $0 list <repository> <tag> | delete <repository> <digest>..." >&2; exit 1 ;;
esac
