"""Classify changed paths conservatively for the CI documentation shortcut."""
import sys


def docs_only(paths):
    return bool(paths) and all(
        path in ('README.md', 'AGENTS.md')
        or (path.startswith('docs/') and path.endswith('.md'))
        for path in paths
    )


if __name__ == '__main__':
    paths = [p.decode('utf-8', errors='surrogateescape') for p in sys.stdin.buffer.read().split(b'\0') if p]
    print('true' if docs_only(paths) else 'false')
