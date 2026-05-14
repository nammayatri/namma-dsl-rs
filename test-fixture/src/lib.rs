// Test fixture; verifies generator output compiles against shared-kernel-rs.

pub mod types {
    pub mod id {
        pub use shared::types::id::Id;
    }
}

pub mod domain {
    pub mod types {
        pub mod seat {
            include!("../src-read-only/domain/types/seat.rs");
        }
        pub mod seat_layout {
            include!("../src-read-only/domain/types/seat_layout.rs");
        }
        pub mod account {
            include!("../src-read-only/domain/types/account.rs");
        }
        pub mod merchant {
            pub struct Merchant; // app-supplied stub
        }
        pub mod merchant_operating_city {
            pub struct MerchantOperatingCity;
        }
    }
}

pub mod storage {
    pub mod schema {
        include!("../src-read-only/storage/schema/seat.rs");
        include!("../src-read-only/storage/schema/seat_layout.rs");
        include!("../src-read-only/storage/schema/account.rs");
    }
    pub mod models {
        pub mod seat {
            include!("../src-read-only/storage/models/seat.rs");
        }
        pub mod seat_layout {
            include!("../src-read-only/storage/models/seat_layout.rs");
        }
        pub mod account {
            include!("../src-read-only/storage/models/account.rs");
        }
    }
    pub mod queries {
        pub mod seat {
            include!("../src-read-only/storage/queries/seat.rs");
        }
        pub mod seat_layout {
            include!("../src-read-only/storage/queries/seat_layout.rs");
        }
        pub mod account {
            include!("../src-read-only/storage/queries/account.rs");
        }
    }
}
