/// Documentation for User struct
/// 
/// # Examples
/// 
/// ```
/// let user = User::new("Alice");
/// assert_eq!(user.name(), "Alice");
/// ```
pub struct User {
    name: String,
}

impl User {
    /// Create a new User
    /// 
    /// # Examples
    /// 
    /// ```
    /// let user = User::new("Bob");
    /// assert_eq!(user.name(), "Bob");
    /// ```
    pub fn new(name: &str) -> Self {
        User {
            name: name.to_string(),
        }
    }
    
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_new_user() {
        let user = User::new("Charlie");
        assert_eq!(user.name(), "Charlie");
    }
}