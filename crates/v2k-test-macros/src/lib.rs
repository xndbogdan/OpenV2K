//! Native libtest ignore markers for optional, private game corpora.

use proc_macro::TokenStream;

/// A retail regression, ignored by libtest when the configured corpus is absent.
/// `retail_test(demo)` additionally requires the separately supplied demo corpus.
#[proc_macro_attribute]
pub fn retail_test(args: TokenStream, item: TokenStream) -> TokenStream {
    let (condition, reason) = match args.to_string().as_str() {
        "" => (
            "not(v2k_retail)",
            "retail game missing: use retail/ or V2K_RETAIL_DIR",
        ),
        "demo" => (
            "not(all(v2k_retail, v2k_demo))",
            "retail/demo comparison data missing: use V2K_RETAIL_DIR and V2K_DEMO_DIR",
        ),
        _ => {
            return "compile_error!(\"expected retail_test or retail_test(demo)\");"
                .parse()
                .unwrap()
        }
    };
    let mut output: TokenStream =
        format!("#[test] #[cfg_attr({condition}, ignore = \"{reason}\")]")
            .parse()
            .unwrap();
    output.extend(item);
    output
}

/// A demo-only regression, independent of whether retail data is installed.
#[proc_macro_attribute]
pub fn demo_test(args: TokenStream, item: TokenStream) -> TokenStream {
    if !args.is_empty() {
        return "compile_error!(\"demo_test takes no arguments\");"
            .parse()
            .unwrap();
    }
    let mut output: TokenStream = "#[test] #[cfg_attr(not(v2k_demo), ignore = \"demo game missing: use demo/ or V2K_DEMO_DIR\")]".parse().unwrap();
    output.extend(item);
    output
}
