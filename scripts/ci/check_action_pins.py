"""Verify each workflow action's pinned commit and entrypoint on GitHub."""
import concurrent.futures
import json
from pathlib import Path
import urllib.error
import urllib.request
import yaml

ROOT = Path(__file__).resolve().parents[2]


def get(url):
    request = urllib.request.Request(url, headers={"User-Agent": "humanymous-action-pin-review"})
    with urllib.request.urlopen(request, timeout=30) as response:
        return response.read()


def verify(action):
    name, sha = action.split("@")
    owner, repository, *path = name.split("/")
    # Commit API rejects a non-existent commit; raw URL checks the specific
    # action entrypoint, including composite/subdirectory actions.
    commit = json.loads(get(f"https://api.github.com/repos/{owner}/{repository}/commits/{sha}"))
    if commit["sha"] != sha:
        raise ValueError(f"commit identity mismatch: {action}")
    prefix = "/".join([owner, repository, sha, *path])
    try:
        entrypoint = get(f"https://raw.githubusercontent.com/{prefix}/action.yml")
    except urllib.error.HTTPError as error:
        if error.code != 404:
            raise
        entrypoint = get(f"https://raw.githubusercontent.com/{prefix}/action.yaml")
    document = yaml.safe_load(entrypoint)
    if "runs" not in document:
        raise ValueError(f"missing action runs entrypoint: {action}")
    return {"action": action, "commit": commit["sha"], "entrypoint": "verified"}


def main():
    actions = set()
    for workflow in (ROOT / ".github/workflows").glob("*.yml"):
        document = yaml.safe_load(workflow.read_text(encoding="utf-8"))
        for job in document["jobs"].values():
            for step in job.get("steps", []):
                if step.get("uses", "").find("@") > 0:
                    actions.add(step["uses"])
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        # list consumes every result; any lookup failure blocks validation.
        results = list(pool.map(verify, sorted(actions)))
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
