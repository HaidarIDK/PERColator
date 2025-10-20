# Deploying PERColator Frontend to Render with Custom Domain

## Step 1: Prepare Your Repository

Make sure all your changes are committed and pushed to GitHub:

```bash
git add .
git commit -m "Prepare for Render deployment"
git push origin master
```

## Step 2: Create Render Account

1. Go to https://render.com
2. Sign up with your GitHub account
3. Authorize Render to access your repositories

## Step 3: Deploy to Render

### Option A: Using Render Dashboard (Recommended)

1. **Click "New +"** in the top right
2. **Select "Web Service"**
3. **Connect your repository**:
   - Select `HaidarIDK/percolator`
   - Click "Connect"

4. **Configure the service**:
   ```
   Name: percolator-frontend
   Region: Oregon (US West)
   Branch: master
   Root Directory: frontend
   Runtime: Node
   Build Command: npm install && npm run build
   Start Command: npm start
   Plan: Free (or Starter for custom domain)
   ```

5. **Add Environment Variables**:
   - Click "Advanced"
   - Add environment variable:
     ```
     Key: NEXT_PUBLIC_API_URL
     Value: https://your-backend-url.onrender.com
     ```
   - Add another:
     ```
     Key: NODE_ENV
     Value: production
     ```

6. **Click "Create Web Service"**
7. Wait for the build to complete (5-10 minutes)

### Option B: Using render.yaml (Automatic)

1. The `render.yaml` file is already created in the `frontend/` directory
2. In Render dashboard, create a new "Blueprint"
3. Select your repository
4. Render will automatically detect the `render.yaml` and configure everything

## Step 4: Connect Your GoDaddy Domain

### In GoDaddy:

1. **Login to GoDaddy**
2. **Go to "My Products" → "DNS"**
3. **Select your domain** (e.g., `percolator.site`)
4. **Click "Add" to create new records**

### Add DNS Records:

**For root domain (percolator.site):**
```
Type: A
Name: @
Value: 216.24.57.1 (Render's IP)
TTL: 600 seconds
```

**For www subdomain:**
```
Type: CNAME
Name: www
Value: <your-render-app>.onrender.com
TTL: 600 seconds
```

**For dex subdomain:**
```
Type: CNAME
Name: dex
Value: <your-render-app>.onrender.com
TTL: 600 seconds
```

### In Render:

1. **Go to your deployed service**
2. **Click "Settings"**
3. **Scroll to "Custom Domain"**
4. **Click "Add Custom Domain"**
5. **Enter your domain**: `percolator.site`
6. **Click "Verify"**
7. **Wait for SSL certificate** (automatic, takes 5-10 minutes)
8. **Repeat for** `www.percolator.site` and `dex.percolator.site`

## Step 5: Update Environment Variables

Once deployed, update your environment variables in Render:

1. Go to your service → "Environment"
2. Update `NEXT_PUBLIC_API_URL` to your actual backend URL
3. Click "Save Changes" (this will trigger a redeploy)

## Step 6: Test Your Deployment

1. Visit `https://percolator.site`
2. Test all pages:
   - Home: `https://percolator.site`
   - Info: `https://percolator.site/info`
   - What I Added: `https://percolator.site/what-i-added`
   - DEX: `https://percolator.site/dex`

3. Verify:
   - Favicon shows correctly
   - All links work
   - Navbar navigation works
   - Images load properly

## Troubleshooting

### Build Fails

If the build fails, check:
- Node version (should be 18+)
- Package.json scripts are correct
- All dependencies are in package.json
- Root directory is set to `frontend`

### Custom Domain Not Working

- Wait 24-48 hours for DNS propagation
- Check DNS records in GoDaddy DNS manager
- Verify SSL certificate in Render (should be "Active")
- Try accessing via `www.percolator.site` instead

### Environment Variables

Update in Render dashboard → Environment tab:
```bash
NEXT_PUBLIC_API_URL=https://your-api.onrender.com
NODE_ENV=production
```

## Additional Configuration

### For Production:

1. **Enable Auto-Deploy**:
   - Settings → Auto-Deploy: ON
   - Every push to `master` triggers deployment

2. **Health Checks**:
   - Render automatically checks if your app is responding
   - Default health check path: `/`

3. **Custom Build Command** (if needed):
   ```bash
   cd frontend && npm install && npm run build
   ```

4. **Custom Start Command** (if needed):
   ```bash
   cd frontend && npm start
   ```

## Cost

- **Free Tier**: 
  - 750 hours/month
  - Spins down after 15 min inactivity
  - First request after spin-down takes 30-60s

- **Starter Tier ($7/month)**:
  - Always running
  - Custom domain support
  - Better for production

## Useful Render Commands

```bash
# View logs
render logs -s percolator-frontend

# Trigger manual deploy
render deploy -s percolator-frontend

# Open service dashboard
render open -s percolator-frontend
```

## Need Help?

- Render Docs: https://render.com/docs
- Next.js on Render: https://render.com/docs/deploy-nextjs
- Custom Domains: https://render.com/docs/custom-domains

---

**Your site will be live at:**
- `https://percolator.site`
- `https://www.percolator.site`
- `https://dex.percolator.site` (Coming Soon page)

