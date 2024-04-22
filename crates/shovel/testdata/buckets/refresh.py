"""Add each known bucket as a Git submodule for testing."""

import json
import subprocess
import sys


def git_submodules() -> list[tuple[str, str, str]]:
    """Retrieve the Git submodules in this repository.

    Returns:
        A list of submodules as a 3-tuple (commit_sha1, path, description).
    """

    output = subprocess.run(
        ["git", "submodule", "status", "."], capture_output=True, text=True
    )

    return [tuple(s.strip().split(maxsplit=3)) for s in output.stdout.splitlines()]


def main() -> int:
    # Get submodule names.
    submodules = set(s[1] for s in git_submodules())

    with open("../../buckets.json", encoding="utf-8") as f:
        buckets: dict[str, str] = json.load(f)

    for bucket, url in buckets.items():
        if bucket not in submodules:
            print(f"[+] Adding bucket {bucket} at {url}")

            # Add the bucket as a submodule with the appropriate path.
            output = subprocess.run(["git", "submodule", "add", url, bucket])
            if code := output.returncode:
                # Bail.
                return code

        else:
            print(f"[-] Skipping bucket {bucket} as it exists")

    return 0


if __name__ == "__main__":
    sys.exit(main())
