"""
Counts the vulnerabilities in the SARIF-files of Docker Scout per severity and generates a
shields.io endpoint-badge for every severity, similar to the vulnerability-overview on Docker Hub.

usage: generate_scout_badges.py <sarif-directory>

Every file '<component>.sarif.json' in the directory results in the badges
'./badges/docker_scout/<component>/<severity>/shields.json'.
"""

import json
import os
import sys

# short name and color of the badge for every severity
SEVERITIES = {
    "critical": ("C", "8b1924"),
    "high": ("H", "e25d33"),
    "medium": ("M", "f89c35"),
    "low": ("L", "fce1a9"),
    "unspecified": ("?", "b0b0b0"),
}

SARIF_SUFFIX = ".sarif.json"


def severity_from_score(score):
    # same mapping of the CVSS-score as by GitHub code scanning
    try:
        score = float(score)
    except (TypeError, ValueError):
        return "unspecified"
    if score >= 9.0:
        return "critical"
    if score >= 7.0:
        return "high"
    if score >= 4.0:
        return "medium"
    if score > 0.0:
        return "low"
    return "unspecified"


def rule_severity(rule):
    props = rule.get("properties", {})

    # severity of the preferred advisory, for example "cvssV3_severity": "HIGH", which is also
    # the one shown by Docker Hub
    for key, value in props.items():
        if key.endswith("_severity") and isinstance(value, str) \
                and value.lower() in SEVERITIES:
            return value.lower()

    for tag in props.get("tags", []):
        if isinstance(tag, str) and tag.lower() in SEVERITIES:
            return tag.lower()

    return severity_from_score(props.get("security-severity"))


def count_vulnerabilities(sarif_path):
    with open(sarif_path) as file:
        sarif = json.load(file)

    counts = dict.fromkeys(SEVERITIES, 0)
    for run in sarif.get("runs", []):
        rules = run.get("tool", {}).get("driver", {}).get("rules", [])
        rules_by_id = {rule.get("id"): rule for rule in rules}

        for result in run.get("results", []):
            rule = rules_by_id.get(result.get("ruleId"))
            index = result.get("ruleIndex")
            if rule is None and isinstance(index, int) and index < len(rules):
                rule = rules[index]
            counts[rule_severity(rule or {})] += 1

    return counts


def write_badges(component, counts):
    for severity, (short_name, color) in SEVERITIES.items():
        path = f"./badges/docker_scout/{component}/{severity}"
        os.makedirs(path, exist_ok=True)

        # without label the badge consists only of the colored message, like on Docker Hub
        data = {
            "schemaVersion": 1,
            "label": "",
            "message": f"{counts[severity]} {short_name}",
            "color": color,
        }

        with open(f"{path}/shields.json", "w") as file:
            json.dump(data, file)


def main():
    sarif_dir = sys.argv[1]

    sarif_files = sorted(f for f in os.listdir(sarif_dir) if f.endswith(SARIF_SUFFIX))
    if not sarif_files:
        sys.exit(f"no SARIF-files found in {sarif_dir}")

    summary = ["| Image | " + " | ".join(SEVERITIES) + " |",
               "| --- |" + " --- |" * len(SEVERITIES)]

    for sarif_file in sarif_files:
        component = sarif_file[:-len(SARIF_SUFFIX)]
        counts = count_vulnerabilities(os.path.join(sarif_dir, sarif_file))
        write_badges(component, counts)

        print(f"{component}: {counts}")
        summary.append(f"| {component} | "
                       + " | ".join(str(counts[s]) for s in SEVERITIES) + " |")

    summary_file = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary_file:
        with open(summary_file, "a") as file:
            file.write("## Docker Scout vulnerabilities\n\n" + "\n".join(summary) + "\n")


if __name__ == "__main__":
    main()
