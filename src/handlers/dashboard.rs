use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

use crate::{capture::log_request, config::AppConfig, events::EventType};

/// Inline HTML for the fake cPanel File Manager dashboard.
const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html><head><title>cPanel - File Manager</title><style>
body{font-family:sans-serif;background:#293a4a;color:#fff;margin:0;padding:20px}
h1{color:#fff;border-bottom:2px solid #ff6c2c;padding-bottom:10px}
.box{background:#fff;color:#333;padding:15px;border-radius:4px;margin:15px 0}
pre{background:#1e1e1e;color:#0f0;padding:10px;border-radius:4px;overflow:auto}
button{background:#ff6c2c;color:#fff;border:none;padding:10px 20px;cursor:pointer;border-radius:3px}
input[type=file]{margin:10px 0}
</style></head>
<body>
<h1>&#9776; cPanel File Manager</h1>
<div class="box">
<h3>Upload File</h3>
<form action="upload" method="post" enctype="multipart/form-data">
<input type="file" name="file" /><br>
<button type="submit">Upload</button>
</form>
</div>
<div class="box">
<h3>Terminal</h3>
<pre id="term">Loading...</pre>
<script>
function runCmd(cmd){
  fetch('term',{method:'POST',body:cmd}).then(r=>r.text()).then(t=>{
    document.getElementById('term').innerText='root@cpanel:~# '+cmd+'\n'+t+'\n';
  });
}
runCmd('whoami');
</script>
</div>
<div class="box">
<h3>Quick Actions</h3>
<button onclick="runCmd('ls -la')">ls -la</button>
<button onclick="runCmd('ps aux')">ps aux</button>
<button onclick="runCmd('netstat -tlnp')">netstat</button>
<button onclick="runCmd('cat /etc/passwd')">passwd</button>
</div>
</body></html>"#;

pub async fn handle_dashboard(
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    State(config): State<Arc<AppConfig>>,
    headers: HeaderMap,
    axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
) -> impl IntoResponse {
    let path = uri.path();
    log_request(&config, EventType::Request, remote, "GET", path, &headers, None).await;
    tracing::info!("[{}] Serving fake dashboard to {}", config.port, remote);

    (
        StatusCode::OK,
        [("Content-Type", "text/html; charset=utf-8")],
        DASHBOARD_HTML,
    )
}
