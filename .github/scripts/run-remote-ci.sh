#/bin/bash
mkdir -m 0700 .ssh
cat > .ssh/id_github_test <<EOF
$SUPER_SECRET
EOF
cat .ssh/id_github_test
head -c 10 .ssh/id_github_test
