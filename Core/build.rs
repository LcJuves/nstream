use std::{error::Error, fs::File, io::Write};

use hyper::{
    body::{Buf, HttpBody},
    Body, Client, Method, Request, Response,
};
use hyper_tls::HttpsConnector;
use serde_json::Value;

async fn get(url: &str) -> Result<Response<Body>, Box<dyn Error>> {
    let req_builder = Request::builder().method(Method::GET).header("User-Agent", "rust").uri(url);
    let client = Client::builder().build::<_, Body>(HttpsConnector::new());
    let req = req_builder.body(Body::empty())?;
    Ok(client.request(req).await?)
}

async fn resp_json_from(resp: Response<Body>) -> Result<Value, Box<dyn Error>> {
    let resp_body = hyper::body::aggregate(resp).await?;
    let mut resp_json_bytes = Vec::new();
    std::io::copy(&mut resp_body.reader(), &mut resp_json_bytes)?;
    let resp_json_string = String::from_utf8_lossy(&resp_json_bytes);
    Ok(serde_json::from_str(&resp_json_string)?)
}

async fn download_maxmind_mmdb() -> Result<(), Box<dyn Error>> {
    let ghurl = "https://api.github.com/repos/Dreamacro/maxmind-geoip/releases";
    let ghapi_resp = get(ghurl).await?;
    let releases_json = resp_json_from(ghapi_resp).await?;
    if let Value::Array(items) = releases_json {
        for item in items {
            if let Value::Bool(prerelease) = &item["prerelease"] {
                if !*prerelease {
                    let asset0 = &item["assets"].as_array().unwrap()[0];
                    let dbname = &asset0["name"].as_str().unwrap();
                    let browser_download_url = &asset0["browser_download_url"].as_str().unwrap();
                    let resp = get(browser_download_url).await?;
                    let location_url = resp.headers().get("location").unwrap().to_str().unwrap();
                    let mut resp = get(location_url).await?;
                    let mut mmdb_file = File::create(dbname)?;
                    while let Some(chunk) = resp.body_mut().data().await {
                        mmdb_file.write_all(&mut chunk?)?;
                    }
                    break;
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/**");

    #[cfg(target_os = "macos")]
    {
        let mut build = cc::Build::new();
        build.include("src/ifname").cpp(false).file("src/ifname/ifname.c");
        build.compile("ifname");
    }

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(download_maxmind_mmdb())?;

    Ok(())
}
