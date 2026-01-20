use dotenv;

pub fn init() {
    dotenv::dotenv().ok();
}

pub fn get(parameter: &str) -> String {
    let env_parameter = std::env::var(parameter)
        .unwrap_or_else(|_| panic!("{} is not defined in the environment.", parameter));
    env_parameter
}
