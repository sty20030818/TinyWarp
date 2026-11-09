use super::premature_shell_message;

#[test]
fn shell_failure_message_preserves_the_shell_detail_verbatim() {
    let detail = "/tmp/中文 {shell} /bin/zsh --login";
    let message = premature_shell_message(detail);
    assert!(message.contains(detail));
    assert!(message.contains("脚本的输出"));
}
