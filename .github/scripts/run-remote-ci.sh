#/bin/bash
mkdir -m 0700 .ssh
cat > .ssh/id_github_test <<EOF
$SUPER_SECRET
EOF
chmod 0600 .ssh/id_github_test
set -x
ssh -vvv -T -i .ssh/id_github_test fedorabook@hoyer.xyz
