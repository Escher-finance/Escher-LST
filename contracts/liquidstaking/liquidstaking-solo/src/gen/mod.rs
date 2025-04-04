pub mod babylon {
    pub mod epoching {
        pub mod v1 {
            include!("babylon.epoching.v1.rs");
        }
    }
}
pub mod cosmos {
    pub mod base {
        pub mod v1beta1 {
            include!("cosmos.base.v1beta1.rs");
        }
    }
    pub mod staking {
        pub mod v1beta1 {
            include!("cosmos.staking.v1beta1.rs");
        }
    }
}
