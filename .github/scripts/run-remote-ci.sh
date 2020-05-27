#/bin/bash
chmod 0700 .ssh
chmod 0600 .ssh/id_github_test
exec ssh -p 22222 -T -o "StrictHostKeyChecking no" -i .ssh/id_github_test fedorabook@hoyer.xyz -- "$GITHUB_TOKEN" "$GITHUB_REPOSITORY" "$GITHUB_SHA" "$GITHUB_REF"
