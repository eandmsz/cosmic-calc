# Test build

A prebuilt `cosmic-calc` so you don't have to compile locally.

| | |
|---|---|
| Source commit | `c214b2d7b4e03153e9d30efea5536b75da774b27` on `claude/codebase-review-n8a46h` |
| Built | 2026-08-14 on Ubuntu 24.04.4 LTS |
| Target | x86-64 Linux |
| **Requires glibc** | **>= 2.39** |

## Run it

```sh
chmod +x cosmic-calc
./cosmic-calc
```

## Will it run on your machine?

Check first — the binary carries a hard glibc floor from the machine
that built it and will refuse to start below it with
`version 'GLIBC_2.39' not found`:

```sh
ldd --version | head -1
```

glibc 2.39 means Ubuntu/Pop!_OS 24.04 or newer, Fedora 40+, or
Debian 13+. On Pop!_OS 22.04 (glibc 2.35) this will **not** start, and
you will need to build from source instead.

Wayland, xkbcommon and a Vulkan driver are opened at runtime rather
than linked, so they need to be present — on a working COSMIC desktop
they already are.

## Why this lives on its own branch

A 20 MB binary in git is permanent: history is immutable, so it cannot
be removed later without a rewrite, and every rebuild would add another
20 MB to every future clone of the repository. Keeping it on
`claude/test-release` — never merged — means the blob stays reachable
only from this branch and disappears the moment you delete it, while
`claude/codebase-review-n8a46h` stays clean to merge.

For an ongoing arrangement, prefer the `binary` CI job on the code
branch: it builds this same artifact on every push to main and attaches
it to the workflow run, with no repository bloat at all.
