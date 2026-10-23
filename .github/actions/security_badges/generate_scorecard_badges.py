"""
Reads the result of the 'Vulnerabilities'-check of the OpenSSF Scorecard, which checks the
dependencies of the project for known vulnerabilities, and generates shields.io endpoint-badges
for its score and its number of vulnerabilities.

usage: generate_scorecard_badges.py <project>, for example github.com/kitsudaiki/ainari

The badges are './badges/scorecard/vulnerabilities/{score,count}/shields.json'.
"""

import json
import os
import sys
import urllib.request

API_URL = "https://api.securityscorecards.dev/projects/{project}"
CHECK_NAME = "Vulnerabilities"
VULNERABILITY_PREFIX = "Warn: Project is vulnerable to:"


def fetch_check(project):
    with urllib.request.urlopen(API_URL.format(project=project), timeout=30) as response:
        result = json.load(response)

    for check in result.get("checks", []):
        if check.get("name") == CHECK_NAME:
            return check
    raise KeyError(f"check '{CHECK_NAME}' not found in the scorecard-result")


def score_color(score):
    if score >= 10:
        return "brightgreen"
    if score >= 5:
        return "f89c35"
    return "8b1924"


def write_badge(name, message, color):
    path = f"./badges/scorecard/vulnerabilities/{name}"
    os.makedirs(path, exist_ok=True)

    data = {
        "schemaVersion": 1,
        "label": "",
        "message": message,
        "color": color,
    }

    with open(f"{path}/shields.json", "w") as file:
        json.dump(data, file)


def main():
    project = sys.argv[1]

    try:
        check = fetch_check(project)
    except Exception as e:
        # the badges of the previous run are kept in the badges-repo
        print(f"::warning::failed to get the scorecard-result of {project}: {e}")
        return

    score = check.get("score", -1)
    vulnerabilities = [d[len(VULNERABILITY_PREFIX):].strip()
                       for d in check.get("details") or []
                       if d.startswith(VULNERABILITY_PREFIX)]

    write_badge("score", f"score {score} / 10", score_color(score))
    write_badge("count",
                f"{len(vulnerabilities)} vulnerabilities",
                "brightgreen" if not vulnerabilities else "8b1924")

    print(f"{CHECK_NAME}: score {score}, {check.get('reason')}")
    for vulnerability in vulnerabilities:
        print(f"  {vulnerability}")


if __name__ == "__main__":
    main()
