//! The Runner of the [Printemps Web Framework](https://www.printempsframework.org/).
//!
//! ## Intro
//!
//! This application is the runtime for running a Printemps web application.
//!
//! See `README.md` and other docs for detailed usage.
//!
//! ## For Printemps users
//!
//! This is just a runner, not the framework itself. Most users don't have to deal with this repository.
//!  
//! To build a Printemps web application, see corresponding packages/repositories in MoonBit language (not ready yet).
//!
//! ## WARNING
//!
//! This software is HIGHLY EXPERIMENTAL and won't reach even `0.1` in a short period.
//!
//! __ANY API IS SUBJECT TO CHANGE, USE AT YOUR OWN RISK.__

mod app;
mod cmdline;
mod moonbit;
mod prelude;

use prelude::*;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

static APP: std::sync::OnceLock<AppHandler> = std::sync::OnceLock::new();

async fn hello(req: hyper::Request<hyper::body::Incoming>) -> Result<BoxBodyResponse> {
    use byteorder::ByteOrder;
    use http_body_util::BodyExt;
    use wasmtime::{ExternRef, Rooted};
    use hyper::header::{HeaderValue, CONTENT_TYPE};
    let app = APP.get().unwrap();

    let mut store = wasmtime::Store::new(&app.wasm_engine, ());
    let inst = app.wasm_linker.instantiate(&mut store, &app.wasm_module)?;

    let hs_path = HostString::new(req.uri().path());
    let hs_extref = ExternRef::new(&mut store, Arc::new(hs_path))?;

    let fn_start = inst.get_typed_func::<(), ()>(&mut store, "_start")?;
    fn_start.call(&mut store, ())?;

    let fn_entrypoint = inst.get_typed_func::<(Rooted<ExternRef>,), (Option<Rooted<ExternRef>>,)>(
        &mut store,
        "http_entrypoint",
    )?;
    let (ret_extern,) = fn_entrypoint.call(&mut store, (hs_extref,))?;
    let ret_moonmem = ret_extern
        .unwrap()
        .data(&mut store)?
        .downcast_ref::<MoonMem>()
        .unwrap();
    let (offset, length) = (ret_moonmem.offset(), ret_moonmem.length());

    let mem = inst.get_memory(&mut store, "moonbit.memory").unwrap();
    let mem_slice = &mem.data(&mut store)[(offset)..(offset + length)];

    let v_u16 = mem_slice
        .chunks_exact(2)
        .map(|chunk| byteorder::LittleEndian::read_u16(&chunk[0..2]))
        .collect::<Vec<_>>();
    let body_str = String::from_utf16(&v_u16)?;
    let body_bytes_v: bytes::Bytes = body_str.into_bytes().into();

    let rsp_body =
        http_body_util::Full::new(body_bytes_v)
            .map_err(|_| anyhow!("unreachable"))
            .boxed();
    let mut rsp = hyper::Response::new(rsp_body);
    rsp.headers_mut().append(CONTENT_TYPE, HeaderValue::from_str("text/plain;charset=utf-8")?);
    Ok(rsp)
}

fn make_wasm_runtime<P: AsRef<Path>>(app: &mut app::AppHandlerInitializing, path: P) -> Result<()> {
    use wasmtime::*;

    let mut option = Config::default();
    option.strategy(Strategy::Auto);
    option.cranelift_debug_verifier(false);

    let engine = Engine::new(&option)?;
    let module = Module::from_file(&engine, path)?;

    let mut linker = Linker::new(&engine);
    linker.func_wrap("spectest", "print_i32", |x: i32| {
        print!("{}", x);
    })?;
    linker.func_wrap("spectest", "print_f64", |x: f64| {
        print!("{}", x);
    })?;
    linker.func_wrap("spectest", "print_char", |x: i32| {
        if let Some(ch) = char::from_u32(x as u32) {
            print!("{}", ch);
        }
    })?;
    unsafe {
        linker.func_new_unchecked(
            "js_string",
            "new",
            FuncType::new(&engine, [ValType::I32, ValType::I32], [ValType::EXTERNREF]),
            |mut caller, space: &mut [ValRaw]| -> Result<()> {
                let param1 = Val::from_raw(&mut caller, space[0], ValType::I32);
                let param2 = Val::from_raw(&mut caller, space[1], ValType::I32);
                assert!(param1.ty(&mut caller).is_i32() && param2.ty(&mut caller).is_i32());
                let (offset, words) = (param1.unwrap_i32() as usize, param2.unwrap_i32());
                println!("js_string new: {} {}", offset, words);

                let bytelen = words as usize * 2;

                let mem = caller.get_export("moonbit.memory").unwrap();
                let mem = mem.into_memory().unwrap();
                let mut buffer = bytes::BytesMut::with_capacity(bytelen);
                buffer.resize(bytelen, 0);
                let buf_view = buffer.as_mut();

                mem.read(&mut caller, offset, &mut buf_view[0..bytelen])?;
                println!("read mem: {:?}", &buffer);

                mem.write(&mut caller, 0, &[0u8])?;
                let b2 = mem.data_mut(&mut caller);
                b2[0] = 0x0;

                let ret = ExternRef::new(&mut caller, MoonMem::new(offset, bytelen))?;
                let ret_raw = ValRaw::externref(ret.to_raw(&mut caller)?);
                space[0] = ret_raw;
                Ok(())
            },
        )?;

        linker.func_new_unchecked(
            "moonmem",
            "id",
            FuncType::new(&engine, [ValType::EXTERNREF], [ValType::I32]),
            |mut caller, space: &mut [ValRaw]| -> Result<()> {
                let param1 = Val::from_raw(&mut caller, space[0], ValType::EXTERNREF);
                assert!(param1.ty(&mut caller).is_externref());
                let externref = param1.unwrap_externref();
                let mm = externref
                    .unwrap()
                    .data(&mut caller)?
                    .downcast_ref::<MoonMem>()
                    .unwrap();
                let ret_raw = ValRaw::i32(mm.id());
                space[0] = ret_raw;
                Ok(())
            },
        )?;

        linker.func_new_unchecked(
            "moonmem",
            "offset",
            FuncType::new(&engine, [ValType::EXTERNREF], [ValType::I32]),
            |mut caller, space: &mut [ValRaw]| -> Result<()> {
                let param1 = Val::from_raw(&mut caller, space[0], ValType::EXTERNREF);
                assert!(param1.ty(&mut caller).is_externref());
                let externref = param1.unwrap_externref();
                let mm = externref
                    .unwrap()
                    .data(&mut caller)?
                    .downcast_ref::<MoonMem>()
                    .unwrap();
                let ret_raw = ValRaw::i32(mm.offset() as i32);
                space[0] = ret_raw;
                Ok(())
            },
        )?;

        linker.func_new_unchecked(
            "moonmem",
            "length",
            FuncType::new(&engine, [ValType::EXTERNREF], [ValType::I32]),
            |mut caller, space: &mut [ValRaw]| -> Result<()> {
                let param1 = Val::from_raw(&mut caller, space[0], ValType::EXTERNREF);
                assert!(param1.ty(&mut caller).is_externref());
                let externref = param1.unwrap_externref();
                let mm = externref
                    .unwrap()
                    .data(&mut caller)?
                    .downcast_ref::<MoonMem>()
                    .unwrap();
                let ret_raw = ValRaw::i32(mm.length() as i32);
                space[0] = ret_raw;
                Ok(())
            },
        )?;

        linker.func_new_unchecked(
            "hoststring",
            "utf16words",
            FuncType::new(&engine, [ValType::EXTERNREF], [ValType::I32]),
            |mut caller, space: &mut [ValRaw]| -> Result<()> {
                let param1 = Val::from_raw(caller.as_context_mut(), space[0], ValType::EXTERNREF);
                assert!(param1.ty(&mut caller).is_externref());
                let externref = param1.unwrap_externref();
                let hs = externref
                    .unwrap()
                    .data(&mut caller)?
                    .downcast_ref::<Arc<HostString>>()
                    .unwrap();
                let ret_raw = ValRaw::i32(hs.utf16_words() as i32);
                space[0] = ret_raw;
                Ok(())
            },
        )?;

        linker.func_new_unchecked(
            "hoststring",
            "fillmem",
            FuncType::new(
                &engine,
                [ValType::EXTERNREF, ValType::I32, ValType::I32],
                [ValType::I32],
            ),
            |mut caller, space: &mut [ValRaw]| -> Result<()> {
                let param1 = Val::from_raw(&mut caller, space[0], ValType::EXTERNREF);
                let param2 = Val::from_raw(&mut caller, space[1], ValType::I32);
                let param3 = Val::from_raw(&mut caller, space[2], ValType::I32);
                assert!(
                    param1.ty(&mut caller).is_externref()
                        && param2.ty(&mut caller).is_i32()
                        && param3.ty(&mut caller).is_i32()
                );
                let externref = param1.unwrap_externref();
                let hs = externref
                    .unwrap()
                    .data(&caller)?
                    .downcast_ref::<Arc<HostString>>()
                    .unwrap();
                let hs = hs.clone();
                let offset = param2.i32().unwrap() as usize;
                let length = param3.i32().unwrap() as usize;

                let mem = caller.get_export("moonbit.memory").unwrap();
                let mem = mem.into_memory().unwrap();
                let mem = mem.data_mut(&mut caller);
                hs.fill_mem(&mut mem[offset..(offset + length)])?;

                let ret_raw = ValRaw::i32(0 as i32);
                space[0] = ret_raw;
                Ok(())
            },
        )?;
    }

    app.wasm_engine = Some(engine);
    app.wasm_module = Some(module);
    app.wasm_linker = Some(linker);
    Ok(())
}

/// Application entrypoint.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use clap::Parser;
    use std::{
        net::{IpAddr, SocketAddr},
        str::FromStr,
    };
    use tokio::net::TcpListener;

    let args = cmdline::Args::parse();
    dbg!(&args);

    let mut app_handler = app::AppHandlerInitializing::new();
    make_wasm_runtime(&mut app_handler, args.wasm_path)?;

    APP.set(AppHandler::new(app_handler))
        .map_err(|_| anyhow!("unable to init app"))?;

    let listen_ip = IpAddr::from_str(&args.listen_addr)?;
    let listen_addr = SocketAddr::from((listen_ip, args.listen_port));

    let listener = TcpListener::bind(listen_addr).await?;

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                println!("accepting connection from {}", &addr);
                let io = hyper_util::rt::TokioIo::new(stream);
                tokio::task::spawn(async move {
                    use hyper::service::service_fn;
                    use hyper_util::rt::TokioTimer;

                    // let ah2 = app_handler.clone();
                    // let service = service_fn(|req: hyper::Request<hyper::body::Incoming>| async move {
                    //     hello(ah2, req).await
                    // });

                    if let Err(err) = hyper::server::conn::http1::Builder::new()
                        .timer(TokioTimer::new())
                        .serve_connection(io, service_fn(hello))
                        .await
                    {
                        eprintln!("Error serving connection: {:?}", err);
                    }
                });
            }
            Err(err) => {
                eprintln!("error accepting incoming connection: {}", &err);
            }
        }
    }
}
