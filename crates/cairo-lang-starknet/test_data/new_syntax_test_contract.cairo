#[starknet::interface]
trait IOtherContract<TStorage> {
    fn decrease_allowed(self: @TStorage) -> bool;
}

#[starknet::interface]
trait ICounterContract<TStorage> {
    fn increase_counter(ref self: TStorage, amount: u128);
    fn decrease_counter(ref self: TStorage, amount: u128);
    fn get_counter(self: @TStorage) -> u128;
}

#[starknet::contract]
mod CounterContract {
    use starknet::ContractAddress;
    use super::{
        IOtherContractDispatcher, IOtherContractDispatcherTrait, IOtherContractLibraryDispatcher
    };

    #[storage]
    struct Storage {
        counter: u128,
        other_contract: IOtherContractDispatcher
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        CounterIncreased: CounterIncreased,
        CounterDecreased: CounterDecreased
    }

    #[derive(Drop, starknet::Event)]
    struct CounterIncreased {
        amount: u128
    }

    #[derive(Drop, starknet::Event)]
    struct CounterDecreased {
        amount: u128
    }

    #[constructor]
    fn init(ref self: Storage, initial_counter: u128, other_contract_addr: ContractAddress) {
        self.counter.write(initial_counter);
        self
            .other_contract
            .write(IOtherContractDispatcher { contract_address: other_contract_addr });
    }

    #[external(v0)]
    impl CounterContract of super::ICounterContract<Storage> {
        fn get_counter(self: @Storage) -> u128 {
            self.counter.read()
        }

        fn increase_counter(ref self: Storage, amount: u128) {
            let current = self.counter.read();
            self.counter.write(current + amount);
            self.emit(Event::CounterIncreased(CounterIncreased { amount }));
        }

        fn decrease_counter(ref self: Storage, amount: u128) {
            let allowed = self.other_contract.read().decrease_allowed();
            if allowed {
                let current = self.counter.read();
                self.counter.write(current - amount);
                self.emit(Event::CounterDecreased(CounterDecreased { amount }));
            }
        }
    }
}

