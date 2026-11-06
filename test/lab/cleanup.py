"""Reap only expired, owned lab projects. Safe to invoke from an external janitor."""
import json
import re
import time
from run import DOCKER, ROOT, COMPOSE
import subprocess


def eligible(labels, now):
    project = labels.get("com.docker.compose.project", "")
    expiry = labels.get("org.humanymous.expires", "")
    return (labels.get("org.humanymous.owner") == "rust-product-lab"
            and re.fullmatch(r"hm-rust-lab-[0-9a-f]{12}", project) is not None
            and expiry.isdecimal() and 0 < int(expiry) <= now)


def command(*args):
    return subprocess.run([DOCKER, *args], cwd=ROOT, check=True, capture_output=True, text=True, timeout=60).stdout


if __name__ == "__main__":
    projects = set()
    now = int(time.time())
    for kind, ls_args in [("container", ("ps", "-aq")), ("network", ("network", "ls", "-q"))]:
        for resource in command(*ls_args, "--filter", "label=org.humanymous.owner=rust-product-lab").splitlines():
            state = json.loads(command(kind, "inspect", resource))[0]
            labels = state["Config"]["Labels"] if kind == "container" else state["Labels"]
            if eligible(labels, now):
                projects.add(labels["com.docker.compose.project"])
    for project in sorted(projects):
        command("compose", "-p", project, "-f", str(COMPOSE), "--profile", "controls", "down", "--volumes", "--remove-orphans", "--timeout", "5")
    print(json.dumps({"expired_owned_projects_removed": sorted(projects)}))
