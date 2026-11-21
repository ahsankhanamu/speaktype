use std::time::Duration;

use bytes::Bytes;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{body_string_contains, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use speaktype_stt::{
    provider_for, timeout_for, CloudService, Decode, MemoryStore, ProviderConfig, SecretStore,
    SttError, TranscribeRequest,
};

const GROQ_MODEL: &str = "whisper-large-v3-turbo";

/// Callers of these cloud-only tests never read the local address; point it at
/// a closed port so an accidental local call fails visibly.
const LOCAL_URL: &str = "http://127.0.0.1:9/inference";

/// The provider sends the multipart `file` part verbatim; nothing on either
/// side of these tests decodes audio, so a compact ASCII stand-in is used
/// (also keeps request bodies valid UTF-8 for `body_string_contains`).
fn wav_bytes() -> Bytes {
    Bytes::from_static(b"RIFF fake 16 kHz mono s16 payload for transport tests")
}

fn request(duration_ms: u32) -> TranscribeRequest {
    TranscribeRequest {
        wav: wav_bytes(),
        duration_ms,
        language: Some("en".to_string()),
        prompt: None,
        decode: Decode::default(),
    }
}

fn verbose_json() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_string(
        r#"{"task":"transcribe","language":"english","duration":2.4,
            "text":" Ship the release notes.\n",
            "segments":[{"id":0,"text":" Ship the release notes.",
            "start":0.16,"end":2.3,"avg_logprob":-0.31,"no_speech_prob":0.012}]}"#,
    )
}

fn cloud(service: CloudService, base_url: String, model: &str) -> ProviderConfig {
    ProviderConfig::Cloud {
        service,
        base_url,
        model: model.to_string(),
        fallback_to_local: false,
    }
}

fn groq_provider(
    server: &MockServer,
    secrets: &MemoryStore,
) -> Result<Box<dyn speaktype_stt::SttProvider>, SttError> {
    let cfg = cloud(
        CloudService::Groq,
        format!("{}/openai/v1", server.uri()),
        GROQ_MODEL,
    );
    provider_for(&cfg, LOCAL_URL, secrets, reqwest::Client::new())
}

#[tokio::test]
async fn openai_sends_bearer_and_provider_model() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/v1/audio/transcriptions"))
        .and(header("authorization", "Bearer sk-test"))
        .and(body_string_contains("name=\"model\""))
        .and(body_string_contains(GROQ_MODEL))
        .and(body_string_contains("name=\"language\""))
        .and(body_string_contains("response_format"))
        .respond_with(verbose_json())
        .expect(1)
        .mount(&server)
        .await;

    let secrets = MemoryStore::new();
    secrets.set("stt.groq", "sk-test").unwrap();
    let provider = groq_provider(&server, &secrets).unwrap();
    let transcript = provider
        .transcribe(request(1000), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(transcript.text, "Ship the release notes.");
}

#[tokio::test]
async fn deepgram_sends_token_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/listen"))
        .and(query_param("model", "nova-3"))
        .and(query_param("language", "en"))
        .and(query_param("smart_format", "true"))
        .and(header("authorization", "Token sk-dg"))
        .and(header("content-type", "audio/wav"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"results":{"channels":[{"alternatives":[{"transcript":" Ship it."}]}]}}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;

    let secrets = MemoryStore::new();
    secrets.set("stt.deepgram", "sk-dg").unwrap();
    let cfg = cloud(
        CloudService::Deepgram,
        format!("{}/v1", server.uri()),
        "nova-3",
    );
    let provider = provider_for(&cfg, LOCAL_URL, &secrets, reqwest::Client::new()).unwrap();
    let transcript = provider
        .transcribe(request(1000), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(transcript.text, "Ship it.");
    assert!(transcript.segments.is_empty());
}

#[tokio::test]
async fn retries_503_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/v1/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(2)
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/openai/v1/audio/transcriptions"))
        .respond_with(verbose_json())
        .expect(1)
        .mount(&server)
        .await;

    let secrets = MemoryStore::new();
    secrets.set("stt.groq", "sk-test").unwrap();
    let provider = groq_provider(&server, &secrets).unwrap();
    let transcript = provider
        .transcribe(request(1000), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(transcript.text, "Ship the release notes.");
}

#[tokio::test]
async fn never_retries_401() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/v1/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&server)
        .await;

    let secrets = MemoryStore::new();
    secrets.set("stt.groq", "sk-test").unwrap();
    let provider = groq_provider(&server, &secrets).unwrap();
    let err = provider
        .transcribe(request(1000), CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(err, SttError::Unauthorized);
}

#[tokio::test]
async fn honours_retry_after_on_429() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/v1/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "1"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/openai/v1/audio/transcriptions"))
        .respond_with(verbose_json())
        .expect(1)
        .mount(&server)
        .await;

    let secrets = MemoryStore::new();
    secrets.set("stt.groq", "sk-test").unwrap();
    let provider = groq_provider(&server, &secrets).unwrap();

    let start = std::time::Instant::now();
    let transcript = provider
        .transcribe(request(1000), CancellationToken::new())
        .await
        .unwrap();
    let elapsed = start.elapsed();
    assert_eq!(transcript.text, "Ship the release notes.");
    assert!(
        elapsed >= Duration::from_secs(1),
        "second request must wait out Retry-After, waited {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "waited {elapsed:?}, which is longer than Retry-After allows"
    );
}

#[tokio::test]
async fn never_waits_longer_than_10s_for_retry_after() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/openai/v1/audio/transcriptions"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "3600"))
        .expect(1)
        .mount(&server)
        .await;

    let secrets = MemoryStore::new();
    secrets.set("stt.groq", "sk-test").unwrap();
    let provider = groq_provider(&server, &secrets).unwrap();

    let started = std::time::Instant::now();
    // Bounded so a regression fails in seconds instead of hanging the suite
    // for the hour the unbounded wait would sleep.
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        provider.transcribe(request(1000), CancellationToken::new()),
    )
    .await
    .expect("transcribe must return instead of waiting out a huge Retry-After");
    assert_eq!(
        result.unwrap_err(),
        SttError::RateLimited {
            retry_after_ms: Some(3_600_000)
        }
    );
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "waited {:?} on a Retry-After beyond the cap",
        started.elapsed()
    );
}

/// A listening socket that never accepts: the provider connects, sends its
/// request, and then waits forever for a response that never comes. Under
/// paused time every idle park advances the clock to the next timer, so the
/// per-attempt timeout fires deterministically without any real waiting.
async fn dead_endpoint() -> (tokio::net::TcpListener, String) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    (listener, format!("http://127.0.0.1:{port}/inference"))
}

#[tokio::test(start_paused = true)]
async fn times_out_and_cancels() {
    assert_eq!(timeout_for(0), Duration::from_secs(10));
    assert_eq!(timeout_for(1000), Duration::from_secs(10));
    assert_eq!(timeout_for(10_000), Duration::from_secs(30));
    assert_eq!(timeout_for(100_000), Duration::from_secs(120));

    let (listener, endpoint) = dead_endpoint().await;
    let cfg = ProviderConfig::Server { base_url: endpoint };
    let provider =
        provider_for(&cfg, LOCAL_URL, &MemoryStore::new(), reqwest::Client::new()).unwrap();
    let err = provider
        .transcribe(request(1000), CancellationToken::new())
        .await
        .unwrap_err();
    drop(listener);
    assert_eq!(err, SttError::Timeout);
}

#[tokio::test(start_paused = true)]
async fn cancels_in_flight_request() {
    let (listener, endpoint) = dead_endpoint().await;
    let cfg = ProviderConfig::Server { base_url: endpoint };
    let provider =
        provider_for(&cfg, LOCAL_URL, &MemoryStore::new(), reqwest::Client::new()).unwrap();
    let cancel = CancellationToken::new();
    let task_cancel = cancel.clone();
    let handle = tokio::spawn(async move {
        provider
            .transcribe(request(1000), task_cancel)
            .await
            .map(|_| ())
    });
    tokio::time::sleep(Duration::from_millis(100)).await;
    cancel.cancel();
    let err = handle.await.unwrap().unwrap_err();
    drop(listener);
    assert_eq!(err, SttError::Cancelled);
}

#[tokio::test]
async fn list_models_filters_to_speech() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/openai/v1/models"))
        .and(header("authorization", "Bearer sk-test"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"data":[{"id":"gpt-4o"},{"id":"gpt-4o-transcribe"},
                {"id":"whisper-1"},{"id":"text-embedding-3-small"}]}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;

    let secrets = MemoryStore::new();
    secrets.set("stt.openai", "sk-test").unwrap();
    let cfg = cloud(
        CloudService::OpenAi,
        format!("{}/openai/v1", server.uri()),
        GROQ_MODEL,
    );
    let provider = provider_for(&cfg, LOCAL_URL, &secrets, reqwest::Client::new()).unwrap();
    let models = provider
        .list_models(CancellationToken::new())
        .await
        .unwrap();
    let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["gpt-4o-transcribe", "whisper-1"]);
}

#[tokio::test]
async fn list_models_falls_back_to_curated_on_404() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/openai/v1/models"))
        .respond_with(ResponseTemplate::new(404))
        .expect(2)
        .mount(&server)
        .await;

    let secrets = MemoryStore::new();
    secrets.set("stt.groq", "sk-test").unwrap();
    secrets.set("stt.openai", "sk-test").unwrap();
    let http = reqwest::Client::new();

    let groq = cloud(
        CloudService::Groq,
        format!("{}/openai/v1", server.uri()),
        GROQ_MODEL,
    );
    let models = provider_for(&groq, LOCAL_URL, &secrets, http.clone())
        .unwrap()
        .list_models(CancellationToken::new())
        .await
        .unwrap();
    let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["whisper-large-v3-turbo", "whisper-large-v3"]);

    let openai = cloud(
        CloudService::OpenAi,
        format!("{}/openai/v1", server.uri()),
        GROQ_MODEL,
    );
    let models = provider_for(&openai, LOCAL_URL, &secrets, http)
        .unwrap()
        .list_models(CancellationToken::new())
        .await
        .unwrap();
    let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(
        ids,
        ["gpt-4o-transcribe", "gpt-4o-mini-transcribe", "whisper-1"]
    );
}

#[tokio::test]
async fn list_models_404_without_curated_errors() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/openai/v1/models"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;

    let secrets = MemoryStore::new();
    secrets.set("stt.custom", "sk-test").unwrap();
    let cfg = cloud(
        CloudService::Custom,
        format!("{}/openai/v1", server.uri()),
        GROQ_MODEL,
    );
    let provider = provider_for(&cfg, LOCAL_URL, &secrets, reqwest::Client::new()).unwrap();
    let err = provider
        .list_models(CancellationToken::new())
        .await
        .unwrap_err();
    assert_eq!(err, SttError::Server { status: 404 });
}

#[tokio::test]
async fn deepgram_list_models_is_curated() {
    let secrets = MemoryStore::new();
    secrets.set("stt.deepgram", "sk-dg").unwrap();
    let cfg = cloud(
        CloudService::Deepgram,
        // Never contacted: list_models is served from the curated list.
        "http://127.0.0.1:9/v1".to_string(),
        "nova-3",
    );
    let provider = provider_for(&cfg, LOCAL_URL, &secrets, reqwest::Client::new()).unwrap();
    let models = provider
        .list_models(CancellationToken::new())
        .await
        .unwrap();
    let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, ["nova-3", "nova-2"]);
}

#[tokio::test]
async fn local_uses_given_url() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/whisper-on-9137/inference"))
        .respond_with(verbose_json())
        .expect(1)
        .mount(&server)
        .await;

    let cfg = ProviderConfig::Local {
        model: "base".to_string(),
    };
    let local_url = format!("{}/whisper-on-9137/inference", server.uri());
    let provider = provider_for(
        &cfg,
        &local_url,
        &MemoryStore::new(),
        reqwest::Client::new(),
    )
    .unwrap();
    let transcript = provider
        .transcribe(request(2400), CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(transcript.text, "Ship the release notes.");
}

#[tokio::test]
async fn local_sends_decode_fields() {
    let server = MockServer::start().await;
    let local_url = format!("{}/inference", server.uri());
    Mock::given(method("POST"))
        .and(path("/inference"))
        .and(body_string_contains("name=\"temperature\"\r\n\r\n0.4\r\n"))
        .and(body_string_contains(
            "name=\"temperature_inc\"\r\n\r\n0.0\r\n",
        ))
        .respond_with(verbose_json())
        .expect(1)
        .mount(&server)
        .await;

    let cfg = ProviderConfig::Local {
        model: "base".to_string(),
    };
    let provider = provider_for(
        &cfg,
        &local_url,
        &MemoryStore::new(),
        reqwest::Client::new(),
    )
    .unwrap();
    let req = TranscribeRequest {
        decode: Decode {
            temperature: 0.4,
            internal_fallback: false,
        },
        ..request(2400)
    };
    let transcript = provider
        .transcribe(req, CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(transcript.text, "Ship the release notes.");
}

#[tokio::test]
async fn local_parses_quality_fields() {
    let server = MockServer::start().await;
    // Fixture trimmed from an actual whisper-server verbose_json response
    // (large-v3, VAD on), as pinned by the transcribe.rs tests.
    Mock::given(method("POST"))
        .and(path("/inference"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"task":"transcribe","language":"english","duration":2.4,
                "text":" Ship the release notes before the stand-up tomorrow.\n",
                "segments":[{"id":0,"text":" Ship the release notes before the stand-up tomorrow.",
                "start":0.16,"end":2.3000000000000003,"temperature":0.0,
                "avg_logprob":-0.04834417253732681,"no_speech_prob":0.01814487762749195}]}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;

    let cfg = ProviderConfig::Server {
        base_url: format!("{}/inference", server.uri()),
    };
    let provider =
        provider_for(&cfg, LOCAL_URL, &MemoryStore::new(), reqwest::Client::new()).unwrap();
    let transcript = provider
        .transcribe(request(2400), CancellationToken::new())
        .await
        .unwrap();

    assert_eq!(
        transcript.text,
        "Ship the release notes before the stand-up tomorrow."
    );
    assert_eq!(transcript.segments.len(), 1);
    let seg = &transcript.segments[0];
    assert_eq!(
        seg.text,
        "Ship the release notes before the stand-up tomorrow."
    );
    assert_eq!(seg.start_ms, 160);
    assert_eq!(seg.end_ms, 2300);
    assert_eq!(seg.no_speech_prob, Some(0.01814487762749195_f64 as f32));
    assert_eq!(seg.avg_logprob, Some(-0.04834417253732681_f64 as f32));
}
