#!/usr/bin/env python3
"""Überträgt `rust/` aus dem Branch `rust-godot` in das eigenständige Repo
github.com:wowseer/forge-rules-rs (Branch `main`).

Entspricht `git subtree split --prefix=rust`, betrachtet aber nur die Commits
seit `master` (dort gibt es kein `rust/`) – Sekunden statt Minuten. Die
erzeugten Commits sind bitgleich zu `subtree split` (gleicher Baum, Autor,
Committer, Datum, Nachricht), daher ist der Push ein Fast-Forward.

Aufruf aus dem Repo-Root:
    python3 rust/tools/sync_github.py            # nur Branch rust-only aktualisieren
    python3 rust/tools/sync_github.py --push     # zusätzlich nach forge-rules-rs/main pushen
"""

import os
import subprocess
import sys

REMOTE = "forge-rules-rs"
REMOTE_URL = "git@github.com:wowseer/forge-rules-rs.git"
BASE = "master"
SOURCE = "rust-godot"
PREFIX = "rust"
TARGET_BRANCH = "rust-only"


def git(*args, env=None, input_bytes=None):
    return subprocess.run(["git", *args], check=True, capture_output=True, env=env, input=input_bytes).stdout


def commit_parts(c):
    raw = git("cat-file", "commit", c)
    header, _, message = raw.partition(b"\n\n")
    fields = {}
    for line in header.decode().splitlines():
        k, _, v = line.partition(" ")
        fields.setdefault(k, v)
    return fields, message


def ident(v):
    # "Name <mail> 1700000000 +0100"
    name, _, rest = v.partition(" <")
    mail, _, date = rest.partition("> ")
    return name, mail, date


def main():
    os.chdir(git("rev-parse", "--show-toplevel").decode().strip())
    commits = git("rev-list", "--reverse", "--topo-order", f"{BASE}..{SOURCE}").decode().split()
    prev_new, prev_tree = None, None
    for c in commits:
        try:
            tree = git("rev-parse", "-q", "--verify", f"{c}:{PREFIX}").decode().strip()
        except subprocess.CalledProcessError:
            continue
        if tree == prev_tree:
            continue
        fields, message = commit_parts(c)
        an, am, ad = ident(fields["author"])
        cn, cm, cd = ident(fields["committer"])
        env = dict(os.environ, GIT_AUTHOR_NAME=an, GIT_AUTHOR_EMAIL=am, GIT_AUTHOR_DATE=ad,
                   GIT_COMMITTER_NAME=cn, GIT_COMMITTER_EMAIL=cm, GIT_COMMITTER_DATE=cd)
        args = ["commit-tree", tree] + (["-p", prev_new] if prev_new else [])
        prev_new = git(*args, env=env, input_bytes=message).decode().strip()
        prev_tree = tree
    if not prev_new:
        sys.exit("keine Commits mit rust/ gefunden")
    git("branch", "-f", TARGET_BRANCH, prev_new)
    print(f"{TARGET_BRANCH} -> {prev_new}")
    if "--push" in sys.argv:
        remotes = git("remote").decode().split()
        if REMOTE not in remotes:
            git("remote", "add", REMOTE, REMOTE_URL)
        out = subprocess.run(["git", "push", REMOTE, f"{TARGET_BRANCH}:main"], capture_output=True, text=True)
        print(out.stdout + out.stderr)
        out.check_returncode()


if __name__ == "__main__":
    main()
