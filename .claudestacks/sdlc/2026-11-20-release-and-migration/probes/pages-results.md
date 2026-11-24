# GitHub Pages and archived repositories

archiving.md (docs.github.com, fetched 2026-11-24) says nothing about Pages:

```text
$ grep -n -i pages docs/archiving.md
exit=1
```

Archived repositories with a workflow-built Pages site still serve it:

```text
microsoft/autogen	true
https://microsoft.github.io/autogen/	workflow
nhn/tui.editor	true
http://nhn.github.io/tui.editor/	legacy
https://microsoft.github.io/autogen/ -> 200
https://nhn.github.io/tui.editor/ -> 200
$ gh api repos/airsstack/claudestacks/pages
https://airsstack.github.io/claudestacks/	workflow
$ gh api repos/airsstack/agentstacks/pages
{"message":"Not Found","documentation_url":"https://docs.github.com/rest/pages/pages#get-a-apiname-pages-site","status":"404"}gh: Not Found (HTTP 404)
https://airsstack.github.io/airsl/install.sh -> 200
https://airsstack.github.io/claudestacks/install.sh -> 200
https://airsstack.github.io/agentstacks/install.sh -> 404
```
