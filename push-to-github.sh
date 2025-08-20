#!/bin/bash

# Script to push the synapsed repository to GitHub

set -e

echo "🚀 Preparing to push Synapsed to GitHub"
echo "========================================"
echo ""

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Not in synapsed root directory${NC}"
    exit 1
fi

echo "📋 Pre-push checklist:"
echo "  1. Have you created the repository at https://github.com/synapsed-me/synapsed?"
echo "  2. Is it set to public or private as desired?"
echo "  3. Do you have push access to the synapsed-me organization?"
echo ""
read -p "Continue? (y/n) " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

# Initialize git if not already done
if [ ! -d ".git" ]; then
    echo "📁 Initializing git repository..."
    git init
    echo -e "${GREEN}✅ Git initialized${NC}"
else
    echo "📁 Git repository already initialized"
fi

# Add .gitignore if it doesn't exist
if [ ! -f ".gitignore" ]; then
    echo "📝 Creating .gitignore..."
    cat > .gitignore << 'EOF'
# Rust
target/
Cargo.lock
**/*.rs.bk
*.pdb

# IDE
.vscode/
.idea/
*.swp
*.swo
*~
.DS_Store

# Environment
.env
.env.local

# Logs
*.log

# Test coverage
*.profraw
*.profdata
coverage/

# Backup files
*.bak
*.backup

# Build artifacts
pkg/
dist/
EOF
    echo -e "${GREEN}✅ .gitignore created${NC}"
fi

# Add all files
echo "📦 Staging all files..."
git add .
echo -e "${GREEN}✅ Files staged${NC}"

# Create initial commit
echo "💾 Creating initial commit..."
git commit -m "Initial commit: Synapsed ecosystem migration from playground

- 16 crates migrated from playground repository
- Observable-first architecture with Substrates and Serventis
- Post-quantum cryptography with GPU acceleration
- Comprehensive networking, storage, and security modules
- Intent verification framework (partial - to be completed)
- CI/CD pipelines configured (not publishing yet)
- Workspace structure validated and building" || echo "Already committed"

# Check if remote exists
if git remote | grep -q "origin"; then
    echo "🔗 Remote 'origin' already exists"
else
    echo "🔗 Adding GitHub remote..."
    echo -e "${YELLOW}Enter the repository URL (or press Enter for default):${NC}"
    echo "Default: https://github.com/synapsed-me/synapsed.git"
    read -r REPO_URL
    
    if [ -z "$REPO_URL" ]; then
        REPO_URL="https://github.com/synapsed-me/synapsed.git"
    fi
    
    git remote add origin "$REPO_URL"
    echo -e "${GREEN}✅ Remote added: $REPO_URL${NC}"
fi

# Show remote info
echo ""
echo "📍 Remote configuration:"
git remote -v
echo ""

# Ask about branch name
echo -e "${YELLOW}Which branch should we push to? (main/master/other)${NC}"
read -r BRANCH_NAME

if [ -z "$BRANCH_NAME" ]; then
    BRANCH_NAME="main"
fi

# Rename branch if needed
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" != "$BRANCH_NAME" ]; then
    echo "🔄 Renaming branch from $CURRENT_BRANCH to $BRANCH_NAME..."
    git branch -m "$BRANCH_NAME"
fi

# Push to GitHub
echo ""
echo "🚀 Pushing to GitHub..."
echo "This will push to: origin/$BRANCH_NAME"
echo ""
read -p "Proceed with push? (y/n) " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    git push -u origin "$BRANCH_NAME"
    echo ""
    echo -e "${GREEN}✅ Successfully pushed to GitHub!${NC}"
    echo ""
    echo "🎉 Your repository is now available at:"
    echo "   https://github.com/synapsed-me/synapsed"
    echo ""
    echo "📋 Next steps:"
    echo "  1. Check the repository on GitHub"
    echo "  2. Set up branch protection rules if desired"
    echo "  3. Add collaborators if needed"
    echo "  4. Configure secrets for GitHub Actions"
    echo "  5. Create initial issues for remaining work"
else
    echo "Push cancelled."
    echo ""
    echo "To push manually later, run:"
    echo "  git push -u origin $BRANCH_NAME"
fi

echo ""
echo "📊 Repository statistics:"
echo "  Crates: $(find crates -name "Cargo.toml" | wc -l)"
echo "  Total files: $(git ls-files | wc -l)"
echo "  Repository size: $(du -sh . | cut -f1)"