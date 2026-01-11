#!/bin/sh
# he apt repository installer
# Usage: curl -fsSL https://sanohiro.github.io/hx/install.sh | sudo sh

set -e

# Add GPG key
curl -fsSL https://sanohiro.github.io/hx/he.gpg | gpg --dearmor -o /usr/share/keyrings/he.gpg

# Add repository
echo "deb [signed-by=/usr/share/keyrings/he.gpg] https://sanohiro.github.io/hx stable main" > /etc/apt/sources.list.d/he.list

# Update package list
apt update

echo "Done! Run 'apt install he' to install."
