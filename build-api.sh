#!/bin/bash
# Vercel build script for API
set -e
echo "Current directory: $(pwd)"
echo "Listing root:"
ls -la
echo "Listing website:"
ls -la website/ || echo "website dir not found"
echo "Changing to website/api..."
cd website/api
echo "Now in: $(pwd)"
npm install
npm run build
