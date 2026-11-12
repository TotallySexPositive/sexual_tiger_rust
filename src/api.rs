use std::net::SocketAddr;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use crate::models::Song;

/// JSON representation of a song matching the schema expected by DataTables:
/// ```javascript
/// $('#songTable').DataTable({
///     "ajax": { "url": "/api/songs", "dataSrc": "" },
///     columns: [
///         { data: 'song_id' },
///         { data: 'name' },
///         { data: 'num_plays' },
///         { data: 'is_clip' },
///         { data: 'duration' },
///         { data: 'username' }
///     ]
/// });
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SongApiDto {
    pub song_id: i64,
    pub name: String,
    pub num_plays: i32,
    pub is_clip: i32,
    pub duration: i32,
    pub username: String,
    pub added_by: Option<String>,
    pub url: Option<String>,
    pub source: String,
    pub last_played: Option<i64>,
}

/// Retrieve all songs from the SQLite database formatted for API / DataTables consumption.
pub async fn get_all_songs_api(pool: &SqlitePool) -> Result<Vec<SongApiDto>, sqlx::Error> {
    let songs = sqlx::query_as::<_, Song>("SELECT * FROM song ORDER BY song_id ASC")
        .fetch_all(pool)
        .await?;

    let dtos = songs
        .into_iter()
        .map(|s| {
            let username = s
                .added_by
                .clone()
                .unwrap_or_else(|| "unknown".to_string());
            SongApiDto {
                song_id: s.song_id.unwrap_or(0),
                name: s.name,
                num_plays: s.num_plays.unwrap_or(0),
                is_clip: s.is_clip.unwrap_or(0),
                duration: s.duration.unwrap_or(0),
                username,
                added_by: s.added_by,
                url: s.url,
                source: s.source,
                last_played: s.last_played,
            }
        })
        .collect();

    Ok(dtos)
}

/// Exports `data.json` to common web root paths (such as `./html/data.json` or `/var/www/html/data.json`).
pub async fn export_data_json(pool: &SqlitePool) -> Result<(), String> {
    let dtos = get_all_songs_api(pool).await.map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&dtos).map_err(|e| e.to_string())?;

    let candidates = [
        "html/data.json",
        "./html/data.json",
        "/var/www/html/data.json",
    ];

    for path in candidates {
        let p = std::path::Path::new(path);
        if let Some(parent) = p.parent() {
            if parent.is_dir() {
                if let Err(e) = std::fs::write(p, &json) {
                    eprintln!("Notice: could not write static file {}: {}", path, e);
                } else {
                    println!("Successfully exported {} songs to {}", dtos.len(), path);
                }
            }
        }
    }

    Ok(())
}

/// Starts the lightweight async HTTP API server.
pub async fn start_api_server(
    pool: SqlitePool,
    host: &str,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .map_err(|e| format!("Invalid address {}:{}: {}", host, port, e))?;

    let listener = TcpListener::bind(addr).await?;
    println!("🌐 Web API server listening on http://{}", addr);
    println!("   Available endpoints:");
    println!("   • http://{}/api/songs   (JSON array for DataTables)", addr);
    println!("   • http://{}/data.json   (drop-in data.json endpoint)", addr);
    println!("   • http://{}/health      (health check & metadata)", addr);

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("API accept error: {}", e);
                continue;
            }
        };

        let pool_clone = pool.clone();
        tokio::spawn(async move {
            let res = tokio::time::timeout(
                Duration::from_secs(10),
                handle_connection(stream, pool_clone),
            )
            .await;

            if let Err(e) = res {
                eprintln!("API connection timed out: {}", e);
            }
        });
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    pool: SqlitePool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut buf = [0u8; 4096];
    let mut bytes_read = 0;

    loop {
        let n = stream.read(&mut buf[bytes_read..]).await?;
        if n == 0 {
            return Ok(());
        }
        bytes_read += n;

        let mut headers = [httparse::EMPTY_HEADER; 32];
        let mut req = httparse::Request::new(&mut headers);
        match req.parse(&buf[..bytes_read]) {
            Ok(httparse::Status::Complete(_)) => {
                let method = req.method.unwrap_or("");
                let path = req.path.unwrap_or("/");
                let clean_path = path.split('?').next().unwrap_or("/");

                match (method, clean_path) {
                    ("OPTIONS", _) => {
                        let response = "HTTP/1.1 204 No Content\r\n\
                            Access-Control-Allow-Origin: *\r\n\
                            Access-Control-Allow-Methods: GET, OPTIONS, HEAD\r\n\
                            Access-Control-Allow-Headers: *\r\n\
                            Access-Control-Max-Age: 86400\r\n\
                            Connection: close\r\n\
                            \r\n";
                        stream.write_all(response.as_bytes()).await?;
                    }
                    ("GET" | "HEAD", "/api/songs" | "/api/songs/" | "/data.json" | "/songs.json") => {
                        let songs = get_all_songs_api(&pool).await?;
                        let body = serde_json::to_string(&songs)?;
                        let body_len = body.len();

                        let header = format!(
                            "HTTP/1.1 200 OK\r\n\
                            Content-Type: application/json; charset=utf-8\r\n\
                            Content-Length: {}\r\n\
                            Access-Control-Allow-Origin: *\r\n\
                            Access-Control-Allow-Methods: GET, OPTIONS, HEAD\r\n\
                            Access-Control-Allow-Headers: *\r\n\
                            Connection: close\r\n\
                            \r\n",
                            body_len
                        );
                        stream.write_all(header.as_bytes()).await?;
                        if method == "GET" {
                            stream.write_all(body.as_bytes()).await?;
                        }
                    }
                    ("GET" | "HEAD", "/" | "/health" | "/api" | "/api/health") => {
                        let songs_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM song")
                            .fetch_one(&pool)
                            .await
                            .unwrap_or(0);
                        let body = serde_json::json!({
                            "status": "ok",
                            "bot": "sexual_tiger",
                            "songs_count": songs_count,
                            "endpoints": ["/api/songs", "/data.json", "/health"]
                        })
                        .to_string();
                        let body_len = body.len();

                        let header = format!(
                            "HTTP/1.1 200 OK\r\n\
                            Content-Type: application/json; charset=utf-8\r\n\
                            Content-Length: {}\r\n\
                            Access-Control-Allow-Origin: *\r\n\
                            Access-Control-Allow-Methods: GET, OPTIONS, HEAD\r\n\
                            Access-Control-Allow-Headers: *\r\n\
                            Connection: close\r\n\
                            \r\n",
                            body_len
                        );
                        stream.write_all(header.as_bytes()).await?;
                        if method == "GET" {
                            stream.write_all(body.as_bytes()).await?;
                        }
                    }
                    _ => {
                        let body = "Not Found\n";
                        let header = format!(
                            "HTTP/1.1 404 Not Found\r\n\
                            Content-Type: text/plain; charset=utf-8\r\n\
                            Content-Length: {}\r\n\
                            Access-Control-Allow-Origin: *\r\n\
                            Connection: close\r\n\
                            \r\n",
                            body.len()
                        );
                        stream.write_all(header.as_bytes()).await?;
                        if method != "HEAD" {
                            stream.write_all(body.as_bytes()).await?;
                        }
                    }
                }
                break;
            }
            Ok(httparse::Status::Partial) => {
                if bytes_read >= buf.len() {
                    return Ok(());
                }
                continue;
            }
            Err(_) => return Ok(()),
        }
    }

    let _ = stream.shutdown().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::establish_connection_pool;

    #[tokio::test]
    async fn test_get_all_songs_api() {
        let pool = establish_connection_pool().await;
        let dtos = get_all_songs_api(&pool).await.expect("Failed to fetch songs");
        assert!(!dtos.is_empty(), "Expected songs in test database");

        // Validate first song matches expected DataTables fields
        let first = &dtos[0];
        assert!(first.song_id > 0);
        assert!(!first.name.is_empty());
        assert!(!first.username.is_empty());

        // Validate JSON serialization produces exact DataTables fields
        let json_val = serde_json::to_value(first).expect("Serialization failed");
        assert!(json_val.get("song_id").is_some());
        assert!(json_val.get("name").is_some());
        assert!(json_val.get("num_plays").is_some());
        assert!(json_val.get("is_clip").is_some());
        assert!(json_val.get("duration").is_some());
        assert!(json_val.get("username").is_some());
    }

    #[tokio::test]
    async fn test_api_server_endpoints() {
        let pool = establish_connection_pool().await;
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind ephemeral test port");
        let local_addr = listener.local_addr().expect("Failed to get local addr");

        let pool_clone = pool.clone();
        tokio::spawn(async move {
            loop {
                if let Ok((stream, _)) = listener.accept().await {
                    let p = pool_clone.clone();
                    tokio::spawn(handle_connection(stream, p));
                }
            }
        });

        let client = reqwest::Client::new();

        // 1. Test /api/songs
        let res_api = client
            .get(format!("http://{}/api/songs", local_addr))
            .send()
            .await
            .expect("GET /api/songs failed");
        assert_eq!(res_api.status(), 200);
        assert_eq!(
            res_api
                .headers()
                .get("access-control-allow-origin")
                .unwrap(),
            "*"
        );
        let list: Vec<SongApiDto> = res_api.json().await.expect("Failed to parse JSON");
        assert!(!list.is_empty());

        // 2. Test /data.json
        let res_data = client
            .get(format!("http://{}/data.json", local_addr))
            .send()
            .await
            .expect("GET /data.json failed");
        assert_eq!(res_data.status(), 200);
        let list_data: Vec<SongApiDto> = res_data.json().await.expect("Failed to parse JSON");
        assert_eq!(list_data.len(), list.len());

        // 3. Test OPTIONS CORS preflight
        let res_options = client
            .request(
                reqwest::Method::OPTIONS,
                format!("http://{}/api/songs", local_addr),
            )
            .send()
            .await
            .expect("OPTIONS failed");
        assert_eq!(res_options.status(), 204);
        assert_eq!(
            res_options
                .headers()
                .get("access-control-allow-origin")
                .unwrap(),
            "*"
        );

        // 4. Test /health
        let res_health = client
            .get(format!("http://{}/health", local_addr))
            .send()
            .await
            .expect("GET /health failed");
        assert_eq!(res_health.status(), 200);
        let health_json: serde_json::Value =
            res_health.json().await.expect("Failed to parse health JSON");
        assert_eq!(health_json["status"], "ok");

        // 5. Test 404
        let res_404 = client
            .get(format!("http://{}/unknown_endpoint", local_addr))
            .send()
            .await
            .expect("GET /unknown failed");
        assert_eq!(res_404.status(), 404);
    }
}
