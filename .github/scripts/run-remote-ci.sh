#/bin/bash
cat > id_github_test <<EOF
$SUPER_SECRET
EOF
ssh -T -i id_github_test fedorabook@hoyer.xyz
