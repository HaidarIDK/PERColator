# 🚀 Deploy PERColator Frontend to Render with GoDaddy Domain

## ✅ Quick Deployment Checklist

- [ ] Commit and push all changes to GitHub
- [ ] Create Render account
- [ ] Deploy frontend to Render
- [ ] Configure custom domain in GoDaddy
- [ ] Add custom domain in Render
- [ ] Test the live site

---

## Step 1: Push Your Code to GitHub

```bash
cd C:\Users\7haid\OneDrive\Desktop\percolator
git add .
git commit -m "Ready for production deployment"
git push origin master
```

---

## Step 2: Create Render Account

1. Go to **https://render.com**
2. Click **"Get Started for Free"**
3. **Sign up with GitHub**
4. Authorize Render to access your repositories

---

## Step 3: Deploy Frontend to Render

### Method 1: Using Render Dashboard (Easiest)

1. **Click "New +"** in top-right corner
2. Select **"Web Service"**
3. Click **"Connect a repository"**
4. Find and select **`HaidarIDK/percolator`**
5. Click **"Connect"**

### Configure Your Service:

```yaml
Name: percolator-frontend
Region: Oregon (US West) - or closest to you
Branch: master
Root Directory: frontend
Runtime: Node
Build Command: npm install && npm run build
Start Command: npm start
Instance Type: Free (or Starter $7/month for always-on)
```

### Environment Variables (Click "Advanced"):

Add these environment variables:

```
NODE_ENV = production
NEXT_PUBLIC_API_URL = http://localhost:3000
```

*(You'll update the API URL later when you deploy the backend)*

6. **Click "Create Web Service"**
7. **Wait 5-10 minutes** for the initial build

---

## Step 4: Configure GoDaddy DNS

### Login to GoDaddy:

1. Go to **https://godaddy.com**
2. Login to your account
3. Click **"My Products"**
4. Find your domain → Click **"DNS"**

### Add DNS Records:

#### For Main Domain (percolator.site):

```
Type: A
Name: @
Value: 216.24.57.1
TTL: 600 seconds
```

#### For WWW Subdomain:

```
Type: CNAME
Name: www
Value: <your-app-name>.onrender.com
TTL: 600 seconds
```

Example: `percolator-frontend-abc123.onrender.com`

#### For DEX Subdomain (dex.percolator.site):

```
Type: CNAME  
Name: dex
Value: <your-app-name>.onrender.com
TTL: 600 seconds
```

**Important:** Get your Render URL from the Render dashboard (e.g., `percolator-frontend-xyz.onrender.com`)

---

## Step 5: Add Custom Domain in Render

1. **In Render dashboard**, go to your deployed service
2. Click **"Settings"** tab
3. Scroll to **"Custom Domain"** section
4. Click **"+ Add Custom Domain"**

### Add These Domains:

1. `percolator.site` (root domain)
2. `www.percolator.site`
3. `dex.percolator.site`

For each domain:
- Click "Add Custom Domain"
- Enter the domain
- Click "Verify"
- Wait for SSL certificate (automatic, 5-10 minutes)
- Status should show "Active" with green checkmark

---

## Step 6: Wait for DNS Propagation

- **Time**: 5 minutes to 48 hours (usually 10-30 minutes)
- **Check status**: https://dnschecker.org
- **Enter your domain** and check if it resolves to Render's IP

---

## Step 7: Test Your Live Site

Once DNS propagates, visit:

1. **https://percolator.site** - Main landing page
2. **https://www.percolator.site** - Should redirect to main
3. **https://dex.percolator.site** - Coming soon page
4. **https://percolator.site/info** - Info page
5. **https://percolator.site/what-i-added** - Features page

### Check:
- ✅ Favicon displays correctly
- ✅ All links work
- ✅ Navbar navigation works
- ✅ Scroll animations work
- ✅ PerpDEX button shows "Soon" badge
- ✅ Scroll-down indicator bounces

---

## Troubleshooting

### Build Fails on Render

**Error: "Module not found"**
```bash
# Solution: Check package.json has all dependencies
cd frontend
npm install
```

**Error: "Out of memory"**
```bash
# Solution: Upgrade to Starter plan ($7/month)
# Or reduce build size in next.config.ts
```

### Custom Domain Not Working

**DNS not propagating:**
- Wait 24-48 hours
- Clear your browser cache (Ctrl+Shift+Delete)
- Try incognito mode
- Check https://dnschecker.org

**SSL Certificate Failed:**
- Verify DNS records in GoDaddy point to Render
- Wait 10-15 minutes for automatic SSL
- Check Render dashboard shows "Active" SSL

**"This site can't be reached":**
- Verify A record points to `216.24.57.1`
- Verify CNAME records point to `<your-app>.onrender.com`
- Check Render service is "Live" (green status)

### Favicon Not Showing

1. Hard refresh: `Ctrl+Shift+R` (or `Cmd+Shift+R` on Mac)
2. Clear browser cache
3. Check `frontend/public/favicon.ico` exists
4. Wait a few minutes for CDN cache to clear

---

## Production Checklist

Before going live:

- [ ] Update `NEXT_PUBLIC_API_URL` to real backend URL
- [ ] Test all pages work
- [ ] Verify mobile responsiveness
- [ ] Check all external links
- [ ] Test navbar on all pages
- [ ] Verify scroll animations
- [ ] Check "Coming Soon" page loads
- [ ] Test GitHub links in "What I Added" page

---

## Render Free Tier Limitations

- **750 hours/month** (free tier)
- **Spins down after 15 min** of inactivity
- **Cold start**: 30-60 seconds for first request
- **No custom domains** on free tier (need Starter $7/month)

### Upgrade to Starter for:
- ✅ Custom domain support
- ✅ Always running (no spin down)
- ✅ Faster response times
- ✅ Better for production use

---

## Auto-Deploy Setup

Enable automatic deployments on every git push:

1. Go to Render dashboard → Your service
2. Click **"Settings"**
3. Scroll to **"Build & Deploy"**
4. Enable **"Auto-Deploy"**: Yes
5. **Branch**: master

Now every `git push origin master` triggers automatic deployment! 🚀

---

## Environment Variables for Production

When you deploy the backend API, update:

```bash
NEXT_PUBLIC_API_URL = https://your-api-domain.com
```

In Render:
1. Settings → Environment
2. Edit `NEXT_PUBLIC_API_URL`
3. Click "Save" (triggers redeploy)

---

## Monitoring Your Deployment

### View Logs:
1. Render Dashboard → Your service
2. Click **"Logs"** tab
3. See real-time deployment and runtime logs

### Check Build Status:
- Green dot = Live and healthy
- Yellow dot = Building
- Red dot = Failed (check logs)

---

## Custom Domain Setup Summary

### In GoDaddy DNS Manager:

| Type  | Name | Value                            | TTL |
|-------|------|----------------------------------|-----|
| A     | @    | 216.24.57.1                     | 600 |
| CNAME | www  | your-app-name.onrender.com      | 600 |
| CNAME | dex  | your-app-name.onrender.com      | 600 |

### In Render Dashboard:

Add these custom domains:
1. `percolator.site`
2. `www.percolator.site`  
3. `dex.percolator.site`

---

## Cost Breakdown

### Free Tier:
- **Cost**: $0/month
- **Hosting**: 750 hours (enough for 1 site)
- **Limitation**: No custom domains, spins down

### Starter Tier:
- **Cost**: $7/month
- **Hosting**: Always running
- **Custom domains**: Unlimited
- **SSL**: Automatic and free
- **Recommended**: For production use

---

## Next Steps After Deployment

1. **Deploy Backend API** (separate Render service)
2. **Update `NEXT_PUBLIC_API_URL`** in frontend
3. **Test trading functionality**
4. **Monitor performance**
5. **Set up monitoring/alerts**

---

## Need Help?

- **Render Docs**: https://render.com/docs
- **Next.js Deployment**: https://render.com/docs/deploy-nextjs
- **Custom Domains**: https://render.com/docs/custom-domains
- **GoDaddy DNS**: https://www.godaddy.com/help/manage-dns-records-680

---

## Your Live URLs (After Deployment)

- 🌐 **Main Site**: https://percolator.site
- 🌐 **WWW**: https://www.percolator.site
- 🌐 **DEX (Coming Soon)**: https://dex.percolator.site
- 🌐 **Info Page**: https://percolator.site/info
- 🌐 **What I Added**: https://percolator.site/what-i-added

---

**Ready to deploy?** Follow the steps above and your site will be live! 🎉

