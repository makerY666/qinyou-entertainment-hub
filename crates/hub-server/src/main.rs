use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut dir = PathBuf::from(".data");
    let mut port = 18765;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--data-dir" => {
                dir = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing --data-dir value"))?
                    .into()
            }
            "--port" => {
                port = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing --port value"))?
                    .parse()?
            }
            "--help" => {
                println!("hub-server [--port 18765] [--data-dir .data]");
                return Ok(());
            }
            _ => anyhow::bail!("unknown argument: {arg}"),
        }
    }
    let host = hub_server::spawn_host(dir, port).await?;
    let info = host.info();
    println!("亲友娱乐 Hub 已启动。局域网加入地址：");
    for address in &info.addresses {
        println!("  {address}");
    }
    println!(
        "本机管理入口：http://127.0.0.1:{}/#admin={}",
        info.port, info.admin_token
    );
    println!("按 Ctrl+C 保存并停止服务。");
    tokio::signal::ctrl_c().await?;
    host.shutdown().await
}
