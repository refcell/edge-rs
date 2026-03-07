---
title: ERC20
---

# ERC20

## Fully Sugared ERC20 Example

```edge
abi ERC20 {
    fn balanceOf(owner: addr) -> u256;
    fn allowance(owner: addr, spender: addr) -> u256;
    fn totalSupply() -> u256;
    fn transfer(receiver: addr, amount: u256) -> bool;
    fn transferFrom(sender: addr, receiver: addr, amount: u256) -> bool;
    fn approve(spender: addr, amount: u256) -> bool;
}

contract MyContract {
    balances: HashMap<addr, u256>,
    allowances: HashMap<addr, HashMap<addr, u256>>,
    supply: u256,
}

impl MyContract: ERC20 {
    type Transfer = event {
        sender: indexed<addr>,
        receiver: indexed<addr>,
        amount: u256,
    }

    type Approval = event {
        owner: indexed<addr>,
        spender: indexed<addr>,
        amount: u256,
    }

    fn balanceOf(self: Self, owner: addr) -> u256 {
        return self.balances.get(owner);
    }

    fn allowance(self: Self, owner: addr, spender: addr) -> u256 {
        return self.allowances.get(owner).get(spender);
    }

    fn totalSupply() -> u256 {
        return self.supply;
    }

    fn transfer(mut self: Self, receiver: addr, amount: u256) -> bool {
        self.balances.set(caller(), storage.balances.get(caller()) - amount);
        self.balances.set(receiver, storage.balances.get(receiver) + amount);
        log(Self::Transfer { sender: caller(), receiver, amount });
        return true;
    }

    fn transferFrom(mut self: Self, sender: addr, receiver: addr, amount: u256) -> bool {
        if sender != caller() {
            let senderCallerAllowance = self.allowances.get(sender).get(caller());
            if senderCallerAllowance < max<u256>() {
                self.allowances.get(sender).set(caller(), senderCallerAllowance - amount);
            }
        }
        self.balances.set(sender, self.balances.get(sender) - amount);
        self.balances.set(receiver, self.balances.get(receiver) + amount);
        log(Self::Transfer { sender, receiver, amount });
        return true;
    }
    fn approve(mut self: Self, spender: addr, amount: u256) -> bool {
        self.allowances.get(caller()).set(spender, amount);
        log(Approval { owner: caller(), spender, amount });
        return true;
    }
}
```

## Fully Desugared ERC20 Example

```edge
type Transfer = event {
    sender: indexed<addr>,
    receiver: indexed<addr>,
    amount: u256,
}

type Approval = event {
    owner: indexed<addr>,
    spender: indexed<addr>,
    amount: u256,
}

abi ERC20 {
    fn balanceOf(owner: addr) -> u256;
    fn allowance(owner: addr, spender: addr) -> u256;
    fn totalSupply() -> u256;
    fn transfer(receiver: addr, amount: u256) -> bool;
    fn transferFrom(sender: addr, receiver: addr, amount: u256) -> bool;
    fn approve(spender: addr, amount: u256) -> bool;
}

type Storage = {
    balances: HashMap<addr, u256>,
    allowances: HashMap<addr, HashMap<addr, u256>>,
    supply: u256,
}

const storage = Storage::default();

fn main<Cd: ERC20>(calldata: Cd) {
    if callvalue() > 0 { revert() };
    match calldata {
        ERC20::balanceOf(owner) => {
            return storage.balances.get(owner);
        },
        ERC20::allowance(owner, spender) => {
            return storage.allowances.get(owner).get(spender);
        },
        ERC20::totalSupply() => {
            return storage.supply;
        },
        ERC20::transfer(receiver, amount) => {
            storage.balances.set(caller(), storage.balances.get(caller()) - amount);
            storage.balances.set(receiver, storage.balances.get(receiver) + amount);
            log(Transfer { sender: caller(), receiver, amount });
            return true;
        },
        ERC20::transferFrom(sender, receiver, amount) => {
            if sender != caller() {
                let senderCallerAllowance = storage.allowances.get(sender).get(caller());
                if senderCallerAllowance < max<u256>() {
                    storage.allowances.get(sender).set(caller(),senderCallerAllowance - amount);
                }
            }
            storage.balances.set(sender, storage.balances.get(sender) - amount);
            storage.balances.set(receiver, storage.balances.get(receiver) + amount);
            log(Transfer { sender, receiver, amount });
            return true;
        },
        ERC20::approve(spender, amount) => {
            storage.allowances.get(caller()).set(spender, amount);
            log(Approval { owner: caller(), spender, amount });
            return true;
        },
        _ => revert(),
    };
}
```
