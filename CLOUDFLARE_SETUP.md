# CloudFlare Setup for gdenv.bytemeadow.com

This guide shows how to set up the CloudFlare Worker to serve install scripts to curl and redirect browsers to GitHub.

## Prerequisites

- CloudFlare account with bytemeadow.com domain
- Wrangler CLI installed (`npm install -g wrangler`)

## Setup Steps

### 1. Create CloudFlare Worker

1. **Login to Wrangler**:
   ```bash
   wrangler login
   ```

2. **Create Worker Project**:
   ```bash
   mkdir gdenv-worker
   cd gdenv-worker
   wrangler init gdenv-worker
   ```

3. **Copy Worker Code**:
   Copy the contents of `cloudflare-worker.js` to `src/worker.js`

### 2. Configure Worker

Create `wrangler.toml`:
```toml
name = "gdenv-worker"
main = "src/worker.js"
compatibility_date = "2024-06-20"

[[routes]]
pattern = "gdenv.bytemeadow.com/*"
zone_name = "bytemeadow.com"
```

### 3. Deploy Worker

```bash
wrangler deploy
```

### 4. Set up DNS Record

In CloudFlare dashboard for bytemeadow.com:

1. Go to **DNS** > **Records**
2. Add **AAAA record**:
   - **Name**: `gdenv`
   - **IPv6**: `100::` (CloudFlare placeholder for Workers)
   - **Proxy status**: Proxied (orange cloud)

### 5. Test Installation

After deployment, test both use cases:

**curl (should serve install script)**:
```bash
curl -fsSL https://gdenv.bytemeadow.com
```

**Browser (should redirect to GitHub)**:
Open https://gdenv.bytemeadow.com in browser

**PowerShell script**:
```powershell
irm https://gdenv.bytemeadow.com/install.ps1
```

## URL Structure

- `https://gdenv.bytemeadow.com/` - Install script for curl, redirect for browsers
- `https://gdenv.bytemeadow.com/install.sh` - Unix install script
- `https://gdenv.bytemeadow.com/install.ps1` - PowerShell install script
- `https://gdenv.bytemeadow.com/health` - Health check endpoint

## Testing Commands

```bash
# Test curl detection
curl -fsSL https://gdenv.bytemeadow.com | head -5

# Test browser-like user agent (should get redirect)
curl -H "User-Agent: Mozilla/5.0" -i https://gdenv.bytemeadow.com

# Test PowerShell script
curl https://gdenv.bytemeadow.com/install.ps1 | head -5

# Test health endpoint
curl https://gdenv.bytemeadow.com/health
```

## Monitoring

The worker includes:
- Error handling for GitHub fetch failures
- Cache headers for performance
- Health check endpoint
- Custom headers for debugging (`x-content-source`)

Monitor in CloudFlare dashboard under **Workers & Pages** > **gdenv-worker**.

## Notes

- Scripts are cached for 5 minutes to balance freshness with performance
- Worker fetches latest scripts from GitHub main branch
- Supports both curl and wget user agents
- CORS enabled for web-based tools that might fetch the scripts