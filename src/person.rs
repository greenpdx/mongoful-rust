struct BItem {
    agency: u16,
    bureau: u8,
    acct:   u32,
    value:  u32,
}
struct Budget {
    bhash: String,
    budget: Vec<BItem>,
}
impl Budget {
    fn new() {
        Budget {
            bhash: '',
            budget<BItem>: Vec::new(),
        }
    }

}

struct Person {
    fname: String,
    lname: String,
    email: String,
    hpass: String,
    salt: String,
    hphone: String,
    age: u8,
    wealth: u8,
    social: u8,
    fiscal: u8,
    saveBudg: Budget,
    submitBudg: Budget,
    temptName: String,
    recipt: String,
}

impl Person {
    fn new() -> Self {
        Person {
            fname: "",
            lname: "",
            email: "",
            hpass: "",
            salt: "",
            hphone: None,
            age: 0,
            wealth: 0,
            social: 0,
            fiscal: 0,
            saveBudg: None,
            submitBudg: None,
            temptName: "",
            recipt: "",
        }
    }
    initPerson(data: Value) {
        
    }
}
