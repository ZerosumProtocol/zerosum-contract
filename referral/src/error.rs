use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("NotExist")]
    NotExist {},

    #[error("AlreadyExist")]
    AlreadyExist {},

    #[error("NotReferral")]
    NotReferral {},

    #[error("NotEnough")]
    NotEnough {},

}
