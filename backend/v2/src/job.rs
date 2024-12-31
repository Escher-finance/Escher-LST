use cosmos_client::client::Rpc;
use cosmos_client::cosmos_sdk::cosmos::bank::v1beta1::query_client::QueryClient;
use cosmos_client::cosmos_sdk::cosmos::bank::v1beta1::QueryBalanceRequest;
use cosmos_client::cosmos_sdk::cosmos::base::v1beta1::Coin;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Job {
    pub rpc_url: String
}

/// The raw balance data from the balance API
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct BalanceResponse {
    /// denomination
    pub denom: String,
    /// the decimal number of coins of a given denomination
    pub amount: String,
}


impl Job {
    pub async fn new(rpc_url: String) -> Self {
        
        Job {
            rpc_url
        }
    }

    // pub async fn get_balance(&self, address: String, denom: String) -> Coin  {

    //     let request: QueryBalanceRequest = QueryBalanceRequest {
    //         address: address.to_string(),
    //         denom: denom.to_string(),
    //     };
    //     let mut query_client = QueryClient::connect(self.rpc_url.clone())
    //         .await
    //         .unwrap();

    //     let response: Coin = query_client.balance(request).await.unwrap()
    //     .into_inner()
    //     .balance
    //     .unwrap()
    //     .try_into()
    //     .unwrap();

    //     response
    // }

    pub async fn cosmos_get_balance(&self, address: String, denom: String) -> cosmos_client::cosmos_sdk::cosmos::base::v1beta1::Coin  {

        let client = Rpc::new(&self.rpc_url).await.unwrap();
        let response = client.bank.balance(&address, &denom).await.unwrap();
        response.balance.unwrap()
    }


}


// unknown variant `tendermint/PubKeyBn254`, expected `tendermint/PubKeyEd25519` at line 1 column 1070