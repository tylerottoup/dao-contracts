use crate::helpers::chain::Chain;
use crate::helpers::helper::create_dao;
use assert_matches::assert_matches;
use cosm_orc::client::error::ClientError;
use cosm_orc::orchestrator::error::ProcessError;
use cosmwasm_std::{to_binary, Addr, CosmosMsg, Decimal, Uint128};
use cw20_stake::msg::{StakedValueResponse, TotalValueResponse};
use cw_core::query::{GetItemResponse, PauseInfoResponse};
use cw_utils::Duration;
use test_context::test_context;
use voting::{deposit::CheckedDepositInfo, threshold::PercentageThreshold, threshold::Threshold};

// #### ExecuteMsg #####

// TODO: Add tests for all cw-core execute msgs

#[test_context(Chain)]
#[test]
#[ignore]
fn execute_execute_admin_msgs(chain: &mut Chain) {
    // if you are not the admin, you cant execute admin msgs:
    let res = create_dao(
        chain,
        None,
        "exc_admin_msgs_create_dao",
        chain.user.addr.clone(),
    );
    let dao = res.unwrap();

    let res = chain.orc.execute(
        "cw_core",
        "exc_admin_msgs_pause_dao_fail",
        &cw_core::msg::ExecuteMsg::ExecuteAdminMsgs {
            msgs: vec![CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                contract_addr: dao.addr,
                msg: to_binary(&cw_core::msg::ExecuteMsg::Pause {
                    duration: Duration::Time(100),
                })
                .unwrap(),
                funds: vec![],
            })],
        },
        &chain.user.key,
    );

    assert_matches!(
        res.unwrap_err(),
        ProcessError::ClientError(ClientError::CosmosSdk { .. })
    );

    let res = chain
        .orc
        .query(
            "cw_core",
            "exc_admin_msgs_pause_dao_query",
            &cw_core::msg::QueryMsg::PauseInfo {},
        )
        .unwrap();
    let res: PauseInfoResponse = res.data().unwrap();

    assert_eq!(res, PauseInfoResponse::Unpaused {});

    // if you are the admin you can execute admin msgs:
    let res = create_dao(
        chain,
        Some(chain.user.addr.clone()),
        "exc_admin_msgs_create_dao_with_admin",
        chain.user.addr.clone(),
    );
    let dao = res.unwrap();

    chain
        .orc
        .execute(
            "cw_core",
            "exc_admin_msgs_pause_dao",
            &cw_core::msg::ExecuteMsg::ExecuteAdminMsgs {
                msgs: vec![CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                    contract_addr: dao.addr,
                    msg: to_binary(&cw_core::msg::ExecuteMsg::Pause {
                        duration: Duration::Height(100),
                    })
                    .unwrap(),
                    funds: vec![],
                })],
            },
            &chain.user.key,
        )
        .unwrap();

    let res = chain
        .orc
        .query(
            "cw_core",
            "exc_admin_msgs_pause_dao",
            &cw_core::msg::QueryMsg::PauseInfo {},
        )
        .unwrap();

    let res: PauseInfoResponse = res.data().unwrap();
    assert_ne!(res, PauseInfoResponse::Unpaused {});
}

#[test_context(Chain)]
#[test]
#[ignore]
fn execute_items(chain: &mut Chain) {
    // add item:
    let res = create_dao(
        chain,
        Some(chain.user.addr.clone()),
        "exc_items_create_dao",
        chain.user.addr.clone(),
    );

    let dao = res.unwrap();

    let res = chain
        .orc
        .query(
            "cw_core",
            "exc_items_get",
            &cw_core::msg::QueryMsg::GetItem {
                key: "meme".to_string(),
            },
        )
        .unwrap();
    let res: GetItemResponse = res.data().unwrap();

    assert_eq!(res.item, None);

    chain
        .orc
        .execute(
            "cw_core",
            "exc_items_set",
            &cw_core::msg::ExecuteMsg::ExecuteAdminMsgs {
                msgs: vec![CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                    contract_addr: dao.addr.clone(),
                    msg: to_binary(&cw_core::msg::ExecuteMsg::SetItem {
                        key: "meme".to_string(),
                        addr: "foobar".to_string(),
                    })
                    .unwrap(),
                    funds: vec![],
                })],
            },
            &chain.user.key,
        )
        .unwrap();

    let res = chain
        .orc
        .query(
            "cw_core",
            "exc_items_set",
            &cw_core::msg::QueryMsg::GetItem {
                key: "meme".to_string(),
            },
        )
        .unwrap();
    let res: GetItemResponse = res.data().unwrap();

    assert_eq!(res.item, Some("foobar".to_string()));

    // remove item:
    chain
        .orc
        .execute(
            "cw_core",
            "exc_items_rm",
            &cw_core::msg::ExecuteMsg::ExecuteAdminMsgs {
                msgs: vec![CosmosMsg::Wasm(cosmwasm_std::WasmMsg::Execute {
                    contract_addr: dao.addr,
                    msg: to_binary(&cw_core::msg::ExecuteMsg::RemoveItem {
                        key: "meme".to_string(),
                    })
                    .unwrap(),
                    funds: vec![],
                })],
            },
            &chain.user.key,
        )
        .unwrap();

    let res = chain
        .orc
        .query(
            "cw_core",
            "exc_items_rm",
            &cw_core::msg::QueryMsg::GetItem {
                key: "meme".to_string(),
            },
        )
        .unwrap();
    let res: GetItemResponse = res.data().unwrap();

    assert_eq!(res.item, None);
}

// #### InstantiateMsg #####

#[test_context(Chain)]
#[test]
#[ignore]
fn instantiate_with_no_admin(chain: &mut Chain) {
    let res = create_dao(chain, None, "inst_dao_no_admin", chain.user.addr.clone());
    let dao = res.unwrap();

    // ensure the dao is the admin:
    assert_eq!(dao.state.admin, dao.addr);
    assert_eq!(dao.state.pause_info, PauseInfoResponse::Unpaused {});
    assert_eq!(
        dao.state.config,
        cw_core::state::Config {
            name: "DAO DAO".to_string(),
            description: "A DAO that makes DAO tooling".to_string(),
            image_url: None,
            automatically_add_cw20s: false,
            automatically_add_cw721s: false
        }
    );
}

#[test_context(Chain)]
#[test]
#[ignore]
fn instantiate_with_admin(chain: &mut Chain) {
    let voting_contract = "cw20_staked_balance_voting";
    let proposal_contract = "cw_proposal_single";

    let res = create_dao(
        chain,
        Some(chain.user.addr.clone()),
        "inst_admin_create_dao",
        chain.user.addr.clone(),
    );
    let dao = res.unwrap();

    // general dao info is valid:
    assert_eq!(dao.state.admin, chain.user.addr);
    assert_eq!(dao.state.pause_info, PauseInfoResponse::Unpaused {});
    assert_eq!(
        dao.state.config,
        cw_core::state::Config {
            name: "DAO DAO".to_string(),
            description: "A DAO that makes DAO tooling".to_string(),
            image_url: None,
            automatically_add_cw20s: false,
            automatically_add_cw721s: false
        }
    );

    let voting_addr = dao.state.voting_module.as_str();
    let prop_addr = dao.state.proposal_modules[0].address.as_str();

    // voting module config is valid:
    chain
        .orc
        .contract_map
        .add_address(voting_contract, voting_addr)
        .unwrap();
    let res = &chain
        .orc
        .query(
            voting_contract,
            "inst_admin_q_stake",
            &cw20_staked_balance_voting::msg::QueryMsg::StakingContract {},
        )
        .unwrap();
    let staking_addr: &str = res.data().unwrap();

    chain
        .orc
        .contract_map
        .add_address("cw20_stake", staking_addr)
        .unwrap();
    let res = chain
        .orc
        .query(
            "cw20_stake",
            "inst_admin_q_val",
            &cw20_stake::msg::QueryMsg::StakedValue {
                address: chain.user.addr.clone(),
            },
        )
        .unwrap();
    let staked_res: StakedValueResponse = res.data().unwrap();
    assert_eq!(staked_res.value, Uint128::new(0));

    let res = chain
        .orc
        .query(
            "cw20_stake",
            "inst_admin_q_cfg",
            &cw20_stake::msg::QueryMsg::GetConfig {},
        )
        .unwrap();
    let config_res: cw20_stake::state::Config = res.data().unwrap();
    assert_eq!(
        config_res.owner,
        Some(Addr::unchecked(
            chain.orc.contract_map.address("cw_core").unwrap()
        ))
    );
    assert_eq!(config_res.manager, None);

    let res = &chain
        .orc
        .query(
            voting_contract,
            "inst_admin_q_tok",
            &cw20_staked_balance_voting::msg::QueryMsg::TokenContract {},
        )
        .unwrap();
    let token_addr: &str = res.data().unwrap();
    assert_eq!(config_res.token_address, token_addr);

    assert_eq!(config_res.unstaking_duration, Some(Duration::Time(1209600)));

    let res = chain
        .orc
        .query(
            "cw20_stake",
            "inst_admin_q_val",
            &cw20_stake::msg::QueryMsg::TotalValue {},
        )
        .unwrap();
    let total_res: TotalValueResponse = res.data().unwrap();
    assert_eq!(total_res.total, Uint128::new(0));

    // proposal module config is valid:
    chain
        .orc
        .contract_map
        .add_address(proposal_contract, prop_addr)
        .unwrap();
    let res = chain
        .orc
        .query(
            proposal_contract,
            "inst_admin_q_cfg",
            &cw_proposal_single::msg::QueryMsg::Config {},
        )
        .unwrap();
    let config_res: cw_proposal_single::state::Config = res.data().unwrap();

    assert_eq!(config_res.min_voting_period, None);
    assert_eq!(config_res.max_voting_period, Duration::Time(432000));
    assert!(!config_res.allow_revoting);
    assert!(config_res.only_members_execute);
    assert_eq!(
        config_res.deposit_info,
        Some(CheckedDepositInfo {
            token: Addr::unchecked(token_addr),
            deposit: Uint128::new(1000000000),
            refund_failed_proposals: true,
        })
    );
    assert_eq!(
        config_res.threshold,
        Threshold::ThresholdQuorum {
            threshold: PercentageThreshold::Majority {},
            quorum: PercentageThreshold::Percent(Decimal::percent(35)),
        }
    );
    assert_eq!(
        config_res.dao,
        chain.orc.contract_map.address("cw_core").unwrap()
    );
}
