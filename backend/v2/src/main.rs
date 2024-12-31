

mod job;
use crate::job::Job;

#[tokio::main]
async fn main() {
    let rpc_url = "https://rpc-testnet-union.nodeist.net".to_string();
    let job = Job::new(rpc_url).await;
    
    let coin = job.cosmos_get_balance("union1vnglhewf3w66cquy6hr7urjv3589srheampz42".into(), "muno".into()).await;
    println!("{:?}", coin);
}
