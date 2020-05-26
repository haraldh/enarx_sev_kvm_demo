#/bin/bash
exec ssh -p 22222 -T -o "StrictHostKeyChecking no" -i .ssh/id_github_test fedorabook@hoyer.xyz -- "$GITHUB_TOKEN"
