use super::CmdResult;

pub fn whmapi(_argv: &[String]) -> CmdResult {
    let body = r#"{
   "metadata" : {
      "version" : 1,
      "reason" : "OK",
      "result" : 1,
      "command" : "version"
   },
   "data" : {
      "version" : "118.0.13"
   }
}
"#;
    CmdResult::ok(body.into())
}

pub fn uapi(_argv: &[String]) -> CmdResult {
    let body = r#"{
   "messages" : null,
   "errors" : null,
   "status" : 1,
   "data" : {}
}
"#;
    CmdResult::ok(body.into())
}

pub fn cpapi2(_argv: &[String]) -> CmdResult {
    let body = r#"<cpanelresult>
  <event><result>1</result></event>
  <data></data>
</cpanelresult>
"#;
    CmdResult::ok(body.into())
}
