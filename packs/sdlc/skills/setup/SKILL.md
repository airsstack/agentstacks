---
name: setup
description: Provision the committed .agentstacks/sdlc/ chain root (prds/, rfcs/, REVIEW.md). Idempotent, never overwrites. Migrates a legacy .claudestacks/sdlc/ tree on explicit approval only. Use when the user says "{{skill setup}}" or asks to set up the SDLC chain in this repository.
invocation: user
---

Provision the agentstacks-sdlc artifact chain in the current repository.
Every step is idempotent: create only what is missing, never overwrite
anything that exists, and report each item as "created" or "already present".

1. If `.agentstacks/sdlc/` exists, it is the chain root: skip to step 4 and
   leave everything under `.claudestacks/sdlc/` untouched.
2. If `.agentstacks/sdlc/` is absent but `.claudestacks/sdlc/` exists, a
   legacy chain was found. List its top-level entries and report them, then
   ask one question with three answers and act only on an explicit answer:
   - **migrate** — copy the whole `.claudestacks/sdlc/` tree byte for byte
     to `.agentstacks/sdlc/` (`mkdir -p .agentstacks`, then copy, then
     `diff -r` to confirm the copy is identical). The legacy tree stays
     exactly as it was: never edited, never deleted.
   - **start empty** — create the new root per step 4 with no chains.
     The legacy tree stays frozen and is never read.
   - **later** — create nothing and stop. The board keeps pointing here
     until this skill runs again.
   Never copy when `.agentstacks/sdlc/` already holds anything: report
   "already present" and leave both trees alone.
3. In the new root, create the directories `.agentstacks/sdlc/prds/` and
   `.agentstacks/sdlc/rfcs/` if missing.
4. In each of the two, create an empty `.gitkeep` file if the directory has
   no committed content (committed-empty directories need the keep file).
5. If `.agentstacks/sdlc/REVIEW.md` does not exist, create it from the
   template in the fenced block of
   `{{var project_dir}}/{{root}}/references/review-policy.md` (the template body
   only, without the fence). If it exists, leave it untouched and report
   "already present".
6. Do NOT write or modify any `.gitignore` — everything under
   `.agentstacks/` is meant for git. Do NOT write, move, or delete anything
   under `.claudestacks/` — it is a frozen snapshot once migrated.
7. Report what was created versus found, then stop. Committing is the
   user's call. Deleting the legacy snapshot, once the new board is
   verified, is the owner's manual choice, never this skill's.
