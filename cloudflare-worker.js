// CloudFlare Worker for gdenv.bytemeadow.com
// Serves install script to curl, redirects browsers to GitHub

export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);
    const userAgent = request.headers.get("user-agent") || "";

    // Check if request is from curl or similar command-line tool
    const isCurlLike =
      userAgent.toLowerCase().includes("curl") ||
      userAgent.toLowerCase().includes("wget") ||
      userAgent.toLowerCase().includes("httpie");
    
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
        } else if (isCurlLike) {
          return await serveInstallScript();
        } else {
          return redirectToGitHub();
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
        "cache-control": "public, max-age=300", // 5 minute cache
        "x-content-source": "github-raw",
        "x-detected-client": "curl-like",
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
  return Response.redirect("https://github.com/bytemeadow/gdenv", 302);
}
