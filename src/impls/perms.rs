/// A permission is defined as the following structure
/// 
/// <namespace>.<permission>
/// 
/// If a user has the * permission, they will have all permissions in that namespace
/// If namespace is global then only the permission is checked. E.g. global.view allows using the view permission in all namespaces
///
/// If a permission has no namespace, it will be assumed to be global
/// 
/// Anything past the <namespace>.<permission> may be ignored or indexed at the discretion of the implementation and is *undefined behaviour*

/// Check if the user has a permission given a set of user permissions and a permission to check for
pub fn has_perm(perms: &Vec<String>, perm: &str) -> bool {
    let mut perm_split = perm.split('.').collect::<Vec<&str>>();

    if perm_split.len() < 2 {
        // Then assume its a global permission on the namespace
        perm_split = vec![perm, "*"];
    }

    let perm_namespace = perm_split[0];
    let perm_name = perm_split[1];

    for user_perm in perms {
        let mut user_perm_split = user_perm.split('.').collect::<Vec<&str>>();

        if user_perm_split.len() < 2 {
            // Then assume its a global permission
            user_perm_split = vec![user_perm, "*"];
        }

        let user_perm_namespace = user_perm_split[0];
        let user_perm_name = user_perm_split[1];

        if (user_perm_namespace == perm_namespace || user_perm_namespace == "global" || perm_namespace == "global") && (user_perm_name == "*" || user_perm_name == perm_name) {
            return true;
        }
    }

    false
}

/// Builds a permission string from a namespace and permission
pub fn build(namespace: &str, perm: &str) -> String {
    format!("{}.{}", namespace, perm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_perm() {
        assert!(has_perm(&vec!["global.*".to_string()], "test")); // global.* implies test[.*]
        assert!(has_perm(&vec!["global.test".to_string()], "rpc.test")); // global.test implies rpc.test
        assert!(!has_perm(&vec!["global.test".to_string()], "rpc.view_bot_queue")); // global.test does not imply rpc.view_bot_queue as global = rpc, test != view_bot_queue
        assert!(has_perm(&vec!["global.*".to_string()], "rpc.view_bot_queue")); // global.* implies rpc.view_bot_queue
        assert!(has_perm(&vec!["rpc.*".to_string()], "rpc.ViewBotQueue")); // rpc.* implies rpc.view_bot_queue
        assert!(!has_perm(&vec!["rpc.BotClaim".to_string()], "rpc.ViewBotQueue")); // rpc.BotClaim does not implies rpc.ViewBotQueue as BotClaim != ViewBotQueue
        assert!(!has_perm(&vec!["apps.*".to_string()], "rpc.ViewBotQueue")); // apps.* does not imply rpc.ViewBotQueue, apps != rpc
        assert!(!has_perm(&vec!["apps.*".to_string()], "rpc.*")); // apps.* does not imply rpc.*, apps != rpc despite the global permission
        assert!(!has_perm(&vec!["apps.test".to_string()], "rpc.test")); // apps.test does not imply rpc.test, apps != rpc despite the permissions being the same
    }
}