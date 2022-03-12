use primitive_types::{H160, H256, U256};
use serde::{Deserialize, Deserializer};
use std::{collections::BTreeMap, io::BufReader, mem::size_of, str::FromStr};

pub mod arithmetic;
pub mod bitwise_logic_operation;
pub mod io_and_flow_operations;
pub mod log;
pub mod performance;
pub mod vm;

fn deserialize_u256<'de, D>(deserializer: D) -> Result<U256, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    U256::from_str(&s).map_err(serde::de::Error::custom)
}

fn deserialize_h256<'de, D>(deserializer: D) -> Result<H256, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    H256::from_str(&s).map_err(serde::de::Error::custom)
}

fn deserialize_h160<'de, D>(deserializer: D) -> Result<H160, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    H160::from_str(&s).map_err(serde::de::Error::custom)
}

fn deserialize_hex_data<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    hex::decode(
        s[0..2]
            .eq("0x")
            .then(|| s[2..].to_string())
            .ok_or("Missing '0x' prefix for hex data")
            .map_err(serde::de::Error::custom)?,
    )
    .map_err(serde::de::Error::custom)
}

fn deserialize_account_storage<'de, D>(deserializer: D) -> Result<BTreeMap<H256, H256>, D::Error>
where
    D: Deserializer<'de>,
{
    let map = <BTreeMap<String, String>>::deserialize(deserializer)?;
    let feel_zeros = |mut val: String| -> Result<String, String> {
        val = val[0..2]
            .eq("0x")
            .then(|| val[2..].to_string())
            .ok_or("Missing '0x' prefix for hex data")?;

        while val.len() < size_of::<H256>() * 2 {
            val = "00".to_string() + &val;
        }
        val = "0x".to_string() + &val;
        Ok(val)
    };
    Ok(map
        .into_iter()
        .map(|(k, v)| {
            (
                H256::from_str(&feel_zeros(k).unwrap()).expect("Can not parse account storage key"),
                H256::from_str(&feel_zeros(v).unwrap()).expect("Can not parse account storage key"),
            )
        })
        .collect())
}

fn deserialize_accounts<'de, D>(deserializer: D) -> Result<BTreeMap<H160, AccountState>, D::Error>
where
    D: Deserializer<'de>,
{
    let map = <BTreeMap<String, AccountState>>::deserialize(deserializer)?;
    Ok(map
        .into_iter()
        .map(|(k, v)| (H160::from_str(&k).unwrap(), v))
        .collect())
}

pub enum NetworkType {
    Istanbul,
    Berlin,
    London,
}

impl<'de> Deserialize<'de> for NetworkType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "Istanbul" => Ok(Self::Istanbul),
            "Berlin" => Ok(Self::Berlin),
            "London" => Ok(Self::London),
            network => Err(format!("Not known network type, {}", network)),
        }
        .map_err(serde::de::Error::custom)
    }
}

#[derive(Deserialize)]
pub struct AccountState {
    #[serde(deserialize_with = "deserialize_u256")]
    pub balance: U256,
    #[serde(deserialize_with = "deserialize_hex_data")]
    pub code: Vec<u8>,
    #[serde(deserialize_with = "deserialize_u256")]
    pub nonce: U256,
    #[serde(deserialize_with = "deserialize_account_storage")]
    pub storage: BTreeMap<H256, H256>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallTransaction {
    #[serde(deserialize_with = "deserialize_hex_data")]
    pub data: Vec<u8>,
    #[serde(deserialize_with = "deserialize_u256")]
    pub gas_limit: U256,
    #[serde(deserialize_with = "deserialize_u256")]
    pub gas_price: U256,
    #[serde(deserialize_with = "deserialize_h160")]
    pub sender: H160,
    #[serde(deserialize_with = "deserialize_h160")]
    pub to: H160,
    #[serde(deserialize_with = "deserialize_u256")]
    pub value: U256,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockHeader {
    #[serde(deserialize_with = "deserialize_h160")]
    pub coinbase: H160,
    #[serde(deserialize_with = "deserialize_u256")]
    pub difficulty: U256,
    #[serde(deserialize_with = "deserialize_u256")]
    pub gas_limit: U256,
    #[serde(deserialize_with = "deserialize_h256")]
    pub hash: H256,
    #[serde(deserialize_with = "deserialize_u256")]
    pub number: U256,
    #[serde(deserialize_with = "deserialize_u256")]
    pub timestamp: U256,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    block_header: BlockHeader,
    transactions: Vec<CallTransaction>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestCase {
    #[serde(deserialize_with = "deserialize_accounts")]
    pre: BTreeMap<H160, AccountState>,
    network: NetworkType,
    genesis_block_header: BlockHeader,
    blocks: Vec<Block>,
    #[serde(deserialize_with = "deserialize_accounts")]
    post_state: BTreeMap<H160, AccountState>,
}

pub trait TestEvmState: Sized {
    fn init_state() -> Self;

    fn validate_account(&self, address: H160, account: AccountState) -> Result<(), String>;

    fn try_apply_chain_id(self, id: U256) -> Result<Self, String>;

    fn try_apply_network_type(self, net_type: NetworkType) -> Result<Self, String>;

    fn try_apply_account(self, address: H160, account: AccountState) -> Result<Self, String>;

    fn try_apply_block_header(self, block_header: BlockHeader) -> Result<Self, String>;

    fn try_apply_transaction(self, tx: CallTransaction) -> Result<Self, String>;

    fn try_apply_accounts<I>(mut self, iter: I) -> Result<Self, String>
    where
        I: Iterator<Item = (H160, AccountState)>,
    {
        for (address, account) in iter {
            self = self.try_apply_account(address, account)?;
        }
        Ok(self)
    }

    fn try_apply_block(mut self, block: Block) -> Result<Self, String> {
        self = self.try_apply_block_header(block.block_header)?;
        for transaction in block.transactions {
            self = self.try_apply_transaction(transaction)?;
        }

        Ok(self)
    }

    fn try_apply_blocks<I>(mut self, iter: I) -> Result<Self, String>
    where
        I: Iterator<Item = Block>,
    {
        for block in iter {
            self = self.try_apply_block(block)?;
        }
        Ok(self)
    }

    fn validate_accounts<I>(&self, iter: I) -> Result<(), String>
    where
        I: Iterator<Item = (H160, AccountState)>,
    {
        for (address, account) in iter {
            self.validate_account(address, account)?;
        }
        Ok(())
    }
}

pub fn run_evm_test<State: TestEvmState>(test: &str) {
    let reader = BufReader::new(test.as_bytes());

    let test: BTreeMap<String, TestCase> =
        serde_json::from_reader(reader).expect("Parse test cases failed");

    for (test_name, test_case) in test {
        println!("\nRunning test: {} ...", test_name);

        let state = State::init_state()
            .try_apply_chain_id(U256::from_str("0xff").unwrap())
            .unwrap()
            .try_apply_network_type(test_case.network)
            .unwrap()
            .try_apply_accounts(test_case.pre.into_iter())
            .unwrap()
            .try_apply_block_header(test_case.genesis_block_header)
            .unwrap()
            .try_apply_blocks(test_case.blocks.into_iter())
            .unwrap();

        state
            .validate_accounts(test_case.post_state.into_iter())
            .unwrap();
    }
}
#[allow(unused_doc_comments)]
pub fn run_evm_tests<State: TestEvmState>() {
    let tests = vec![
        arithmetic::ADD,
        arithmetic::ADD_MOD,
        arithmetic::ARITH,
        arithmetic::DIV,
        arithmetic::DIV_BY_ZERO,
        arithmetic::EXP,
        arithmetic::EXP_POWER_2,
        arithmetic::EXP_POWER_256,
        arithmetic::EXP_POWER_256_F_256,
        arithmetic::FIB,
        arithmetic::MOD,
        arithmetic::MUL,
        arithmetic::MUL_MOD,
        arithmetic::NOT,
        arithmetic::SDIV,
        arithmetic::SUB,
        arithmetic::TWO_OPS,
        bitwise_logic_operation::AND,
        bitwise_logic_operation::BYTE,
        bitwise_logic_operation::EQ,
        bitwise_logic_operation::GT,
        bitwise_logic_operation::IS_ZERO,
        bitwise_logic_operation::LT,
        bitwise_logic_operation::NOT,
        bitwise_logic_operation::OR,
        bitwise_logic_operation::SGT,
        bitwise_logic_operation::SLT,
        bitwise_logic_operation::XOR,
        io_and_flow_operations::CODE_COPY,
        io_and_flow_operations::GAS,
        io_and_flow_operations::JUMP,
        io_and_flow_operations::JUMPI,
        /// "jumpToPush.json" has a different structure it is does not have a 'postState' field
        /// as a final state which we would have as a result of the test execution.
        /// "jumpToPush.json" test has only "postStateHash" which should be equal to the Ethereum "stateRoot"
        /// which we dont need to implement in our implementation currently as we are not emulating Ethereum blockchain structure
        // io_and_flow_operations::JUMP_TO_PUSH,
        io_and_flow_operations::LOOP_STACK_LIMIT,
        io_and_flow_operations::LOOPS_CONDITIONALS,
        io_and_flow_operations::MLOAD,
        io_and_flow_operations::MSIZE,
        io_and_flow_operations::MSTORE,
        io_and_flow_operations::MSTORE_8,
        io_and_flow_operations::PC,
        io_and_flow_operations::POP,
        io_and_flow_operations::RETURN,
        io_and_flow_operations::SSTORE_SLOAD,
        log::LOG0,
        log::LOG1,
        log::LOG2,
        log::LOG3,
        log::LOG4,
        /// Heavy perfomance tests, so we just skip them
        // performance::LOOP_EXP,
        // performance::LOOP_MUL,
        // performance::PERFOMANCE_TESTER,
        vm::BLOCK_INFO,
        vm::CALL_DATA_COPY,
        vm::CALL_DATA_LOAD,
        vm::CALL_DATA_SIZE,
        vm::DUP,
        vm::ENV_INFO,
        vm::PUSH,
        vm::RANDOM,
        vm::SHA3,
        vm::SUICIDE,
        vm::SWAP,
    ];

    for test in tests {
        run_evm_test::<State>(test);
    }
}

#[cfg(test)]
mod test {

    use super::*;

    struct EvmState;

    impl TestEvmState for EvmState {
        fn init_state() -> Self {
            Self
        }

        fn validate_account(&self, _: H160, _: AccountState) -> Result<(), String> {
            Ok(())
        }

        fn try_apply_chain_id(self, _: U256) -> Result<Self, String> {
            Ok(self)
        }

        fn try_apply_network_type(self, _: NetworkType) -> Result<Self, String> {
            Ok(self)
        }

        fn try_apply_account(self, _: H160, _: AccountState) -> Result<Self, String> {
            Ok(self)
        }

        fn try_apply_block_header(self, _: BlockHeader) -> Result<Self, String> {
            Ok(self)
        }

        fn try_apply_transaction(self, _: CallTransaction) -> Result<Self, String> {
            Ok(self)
        }
    }

    #[test]
    fn run_tests() {
        run_evm_tests::<EvmState>();
    }
}
