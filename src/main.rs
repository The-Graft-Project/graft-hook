use axum::{extract::State, routing::post, Json, Router};
use bollard::auth::DockerCredentials;
use bollard::image::CreateImageOptions;
use bollard::Docker;
use git2::Repository;
use serde::Deserialize;
use std::{env, sync::Arc};
use futures_util::stream::StreamExt;
use dotenvy; // This allows using dotenvy::dotenv()

#[derive(Deserialize)]
struct WebhookPayload {
    project: String,
    githubtoken: String,
    user: String,
    r#type: String, // "repo" or "image"
}

#[derive(Deserialize, Clone)]
struct ProjectConfig {
    name: String,
    path: String,
}

#[derive(Deserialize, Clone)]
struct ConfigFile {
    projects: Vec<ProjectConfig>,
}

struct AppState {
    config: ConfigFile,
    docker: Docker,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    
    let config_path = env::var("configpath").expect("ENV 'configpath' not set");
    let config_content = std::fs::read_to_string(config_path).expect("Failed to read config");
    let config: ConfigFile = serde_json::from_str(&config_content).unwrap();

    // Connect to Docker via Unix Socket
    let docker = Docker::connect_with_unix_defaults().expect("Failed to connect to Docker");

    let state = Arc::new(AppState { config, docker });

    let app = Router::new().route("/webhook", post(handle_deploy)).with_state(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    
    axum::serve(listener, app).await.unwrap();
}

async fn handle_deploy(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<WebhookPayload>,
) -> &'static str {
    let project = match state.config.projects.iter().find(|p| p.name == payload.project) {
        Some(p) => p,
        None => return "Project not found",
    };

    if payload.r#type == "repo" {
        // Use git2 for native performance
        if let Ok(repo) = Repository::open(&project.path) {
            let mut remote = repo.find_remote("origin").unwrap();
            remote.fetch(&["main"], None, None).unwrap();
            return "Git Fetch Complete";
        }
    } else if payload.r#type == "image" {
        // Use bollard for native Docker Pull
        let auth = DockerCredentials {
            username: Some(payload.user),
            password: Some(payload.githubtoken),
            serveraddress: Some("ghcr.io".to_string()),
            ..Default::default()
        };

        let mut stream = state.docker.create_image(
            Some(CreateImageOptions { from_image: project.name.clone(), ..Default::default() }),
            None,
            Some(auth),
        );

        while let Some(_) = stream.next().await {}
        return "Docker Image Pulled nativesly";
    }

    "Task Started"
}