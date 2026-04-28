use std::{ffi::OsString, path::Path};

pub(crate) fn rewrite_args_for_alias(args: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    let args = args.into_iter().collect::<Vec<_>>();
    let Some(program) = args.first() else {
        return vec![OsString::from("hj")];
    };
    let Some(name) = Path::new(program)
        .file_name()
        .and_then(|value| value.to_str())
    else {
        return args;
    };

    let subcommand = match name {
        "handoff" => Some("handoff"),
        "handon" => Some("handon"),
        "handover" => Some("handover"),
        "handoff-detect" => Some("detect"),
        "handoff-db" => Some("handoff-db"),
        "handup" => Some("handup"),
        _ => None,
    };

    let Some(subcommand) = subcommand else {
        return args;
    };

    let mut rewritten = vec![OsString::from("hj"), OsString::from(subcommand)];
    rewritten.extend(args.into_iter().skip(1));
    rewritten
}
