# Percolator Frontend - Vercel Deployment Guide

## Prerequisites
- Vercel account
- Vercel CLI installed: `npm i -g vercel`
- API backend deployed and accessible

## Deployment Steps

### 1. Install Dependencies
```bash
cd website/frontend
npm install
```

### 2. Set Up Environment Variables
Copy `.env.example` to `.env.production` and configure:
```bash
cp .env.example .env.production
```

Required environment variables:
- `NEXT_PUBLIC_API_URL` - Your API backend URL
- `NEXT_PUBLIC_WS_URL` - WebSocket URL for real-time data
- `NEXT_PUBLIC_SOLANA_NETWORK` - Solana network
- `NEXT_PUBLIC_SOLANA_RPC_URL` - Solana RPC endpoint

### 3. Test Build Locally
```bash
npm run build
npm start
```

Visit http://localhost:3000 to verify the frontend builds correctly.

### 4. Deploy to Vercel
```bash
vercel --prod
```

Or use the Vercel dashboard:
1. Go to https://vercel.com/new
2. Import the `website/frontend` directory
3. Vercel will auto-detect Next.js
4. Configure environment variables
5. Deploy

### 5. Configure Environment Variables in Vercel
In the Vercel dashboard, add these environment variables:
- `NEXT_PUBLIC_API_URL` - Your deployed API URL
- `NEXT_PUBLIC_WS_URL` - WebSocket endpoint
- `NEXT_PUBLIC_SOLANA_NETWORK`
- `NEXT_PUBLIC_SOLANA_RPC_URL`

## Post-Deployment

### Update API CORS
After deploying the frontend, update your API's environment variables:
1. Add your frontend URL to `FRONTEND_URL`
2. Redeploy the API if needed

### Configure Custom Domain (Optional)
1. Go to your Vercel project settings
2. Navigate to "Domains"
3. Add your custom domain
4. Update DNS records as instructed

## Testing
After deployment:
1. Visit your frontend URL
2. Check browser console for errors
3. Verify API connectivity
4. Test wallet connection
5. Test trading functionality

## Troubleshooting
- **API not connecting**: Check CORS settings and API URL
- **Wallet issues**: Verify Solana network matches between frontend and wallet
- **Build errors**: Check Next.js version compatibility