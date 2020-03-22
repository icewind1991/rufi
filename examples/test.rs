use rufi::MenuApp;

pub const WIN_W: u32 = 600;

fn main() {
    let app: MenuApp<String> = MenuApp::new(WIN_W, "Rufi test");

    app.main_loop(|query| {
        let mut acc = vec![];
        let mut result: Vec<String> = vec![];
        for char in query.chars() {
            acc.push(char);
            result.push(acc.clone().into_iter().collect());
        }

        result
    });

    std::thread::sleep(std::time::Duration::from_secs(1));
}
