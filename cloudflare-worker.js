// CloudFlare Worker for gdenv.bytemeadow.com
// Serves install script to curl, redirects browsers to GitHub

export default {
  async fetch(request, env, ctx) {
    // Force no caching at the edge
    ctx.passThroughOnException();
    const url = new URL(request.url);
    const userAgent = request.headers.get("user-agent") || "";

    // Check if request is from a browser (be explicit about browsers)
    const isBrowser = 
      userAgent.toLowerCase().includes("mozilla") ||
      userAgent.toLowerCase().includes("chrome") ||
      userAgent.toLowerCase().includes("safari") ||
      userAgent.toLowerCase().includes("firefox") ||
      userAgent.toLowerCase().includes("edge") ||
      userAgent.toLowerCase().includes("opera") ||
      userAgent.toLowerCase().includes("webkit");
    
    // Check if request is from PowerShell (Windows)
    const isPowerShell = 
      userAgent.toLowerCase().includes("powershell") ||
      userAgent.toLowerCase().includes("microsoft.powershell") ||
      userAgent.toLowerCase().includes("invoke-webrequest") ||
      userAgent.toLowerCase().includes("invoke-restmethod");

    // Handle different paths
    switch (url.pathname) {
      case "/":
      case "/install.sh":
        if (isPowerShell) {
          return await servePowerShellScript();
        } else if (isBrowser) {
          return redirectToGitHub();
        } else {
          // Default to serving script for all non-browser requests
          return await serveInstallScript();
        }

      case "/install.ps1":
        return await servePowerShellScript();

      case "/health":
        return new Response("OK", { status: 200 });

      default:
        return redirectToGitHub();
    }
  },
};

async function serveInstallScript() {
  try {
    // Fetch install script from GitHub
    const response = await fetch(
      "https://raw.githubusercontent.com/bytemeadow/gdenv/main/install.sh",
    );

    if (!response.ok) {
      throw new Error(`GitHub fetch failed: ${response.status}`);
    }

    const script = await response.text();

    return new Response(script, {
      status: 200,
      headers: {
        "content-type": "text/x-sh",
        "cache-control": "no-cache, no-store, must-revalidate", // Disable caching
        "pragma": "no-cache",
        "expires": "0",
        "vary": "User-Agent", // Cache varies by user-agent
        "x-content-source": "github-raw",
        "x-detected-client": "curl-like",
        "x-user-agent": userAgent,
        "access-control-allow-origin": "*",
      },
    });
  } catch (error) {
    return new Response(`# Error fetching install script: ${error.message}`, {
      status: 500,
      headers: {
        "content-type": "text/plain",
      },
    });
  }
}

async function servePowerShellScript() {
  try {
    // Fetch PowerShell script from GitHub
    const response = await fetch(
      "https://raw.githubusercontent.com/bytemeadow/gdenv/main/install.ps1",
    );

    if (!response.ok) {
      throw new Error(`GitHub fetch failed: ${response.status}`);
    }

    const script = await response.text();

    return new Response(script, {
      status: 200,
      headers: {
        "content-type": "text/plain",
        "cache-control": "public, max-age=300", // 5 minute cache
        "x-content-source": "github-raw",
        "x-detected-client": "powershell",
        "access-control-allow-origin": "*",
      },
    });
  } catch (error) {
    return new Response(
      `# Error fetching PowerShell script: ${error.message}`,
      {
        status: 500,
        headers: {
          "content-type": "text/plain",
        },
      },
    );
  }
}

function redirectToGitHub() {
  return new Response("", {
    status: 302,
    headers: {
      "Location": "https://github.com/bytemeadow/gdenv",
      "cache-control": "no-cache, no-store, must-revalidate",
      "pragma": "no-cache", 
      "expires": "0",
      "vary": "User-Agent",
      "x-detected-client": "browser",
    }
  });
}
