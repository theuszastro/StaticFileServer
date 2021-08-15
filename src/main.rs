

use std::convert::Infallible;
use std::fs::{read, File};
use std::io::{prelude::*, Error, SeekFrom};
use std::path::Path;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server};

// use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() -> Result<(), Error> {
   let addr = ([127, 0, 0, 1], 3333).into();

   let make_svc = make_service_fn(|_| async { Ok::<_, Infallible>(service_fn(handle)) });
   let server = Server::bind(&addr).serve(make_svc);

   if let Err(e) = server.await {
      eprintln!("Server Error: {}", e);
   }

   Ok(())
}



async fn get_path(req: &Request<Body>) -> (&Method, String, Vec<String>) {
   let path = req.uri().path().to_string();
   let method = req.method();

   let path_splited: Vec<String> = path
      .split("/")
      .filter(|data| data.len() >= 1)
      .map(|item| item.to_string())
      .collect();

   let path = format!(
      "/{}",
      if path_splited.len() < 1 {
         "/"
      } else {
         path_splited[0].as_str()
      }
   );

   (method, path, path_splited)
}

fn create_error(reason: String, code: u16) -> Result<Response<Body>, Infallible> {
   Ok(Response::builder()
      .status(code)
      .header("Access-Control-Allow-Origin", "*")
      .body(Body::from(reason))
      .unwrap())
}

async fn send_video(
   req: Request<Body>,
   filename: String,
   extention: String,
) -> Result<Response<Body>, Infallible> {
   let range_header = req.headers().get("range");
   let range = if range_header.is_some() {
      let current_range = range_header.unwrap().to_str().unwrap();
      let range_string = current_range.to_string();
      let range_formated = range_string.replace("bytes=", "");
      let range_splited = range_formated
         .split("-")
         .map(|item| String::from(item))
         .collect::<Vec<String>>();

      range_splited[0].clone().parse::<u64>().unwrap()
   } else {
      0
   };

   let path = format!("files/{}", filename);
   let file_path = Path::new(path.as_str());
   let metadata = file_path.metadata();

   let size = if metadata.is_ok() {
      metadata.unwrap().len()
   } else {
      0u64
   };

   let file = File::open(file_path);
   if file.is_ok() {
      let mut file = file.unwrap();
      let mut buf = vec![0; 1024 * 1024 * 2];

      if range != 0 {
         file.seek(SeekFrom::Start(range)).unwrap();
      }
      file.read_exact(&mut buf).unwrap();

      let end = size - 1;

      return Ok(Response::builder()
         .status(206)
         .header("Content-Type", format!("video/{}", extention))
         .header("Content-Range", format!("bytes {}-{}/{}", range, end, size))
         .header("Access-Control-Allow-Origin", "*")
         .header("Accept-Ranges", "bytes")
         .body(Body::from(buf))
         .unwrap());
   }

   create_error(String::from("this file not exists"), 400)
}

async fn send_image(filename: String, extention: String) -> Result<Response<Body>, Infallible> {
   let path = format!("files/{}", filename);
   let file_path = Path::new(path.as_str());

   let new_extention = match extention.as_str() {
      "svg" => "svg+xml",
      _ => extention.as_str(),
   };

   let data = read(file_path);
   if data.is_ok() {
      let data = data.unwrap();

      return Ok(Response::builder()
         .status(200)
         .header("Content-Type", format!("image/{}", new_extention))
         .header("Access-Control-Allow-Origin", "*")
         .body(Body::from(data))
         .unwrap());
   }

   create_error(String::from("this file not exists"), 400)
}

async fn send_audio(filename: String) -> Result<Response<Body>, Infallible> {
   let path = format!("files/{}", filename);
   let file_path = Path::new(path.as_str());

   let data = read(file_path);
   if data.is_ok() {
      let data = data.unwrap();

      return Ok(Response::builder()
         .status(200)
         .header("Content-Type", "audio/mp3")
         .header("Access-Control-Allow-Origin", "*")
         .body(Body::from(data))
         .unwrap());
   }

   create_error(String::from("this file not exists"), 400)
}


async fn handle(req: Request<Body>) -> Result<Response<Body>, Infallible> {
   let (method, path, splited) = get_path(&req).await;

   let splited_len = splited.len();

   match (method, path.as_str()) {
      (&Method::GET, "/file") if splited_len >= 1 => {
         let filename = &splited[1];
         let filename_splited = filename.split('.');
         let extention = filename_splited.last();

         if extention.is_none() {
            return create_error(String::from("this file not exists"), 404);
         }

         let extention = extention.unwrap();

         match extention {
            "mp3" => send_audio(filename.clone()).await,
            "mp4" => send_video(req, filename.clone(), String::from(extention)).await,
            "jpeg" | "jpg" | "png" | "svg" => {
               send_image(filename.clone(), String::from(extention)).await
            }
            _ => create_error(String::from(""), 404),
         }
      }
      _ => Ok(Response::builder()
         .status(404)
         .body(Body::from("this router not exists"))
         .unwrap()),
   }
}
