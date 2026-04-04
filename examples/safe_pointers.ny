// Null safety with ?T optional types
//
// Optional pointers prevent null dereference at compile time.
// Use `??` for defaults and `if let` for safe unwrapping.

struct User {
    name: i32,
    age: i32,
}

fn find_user(id: i32) -> ?*User {
    if id == 42 {
        return new User { name: 42, age: 30 };
    }
    return nil;
}

fn main() -> i32 {
    // found is ?*User — might be nil
    found := find_user(42);
    missing := find_user(99);

    // if let: safe unwrap — only runs if non-nil
    if let user = found {
        println("found user, age:", user.age);
    } else {
        println("not found");
    }

    // ?? null coalescing: provide default
    default_user := new User { name: 0, age: 0 };
    user := missing ?? default_user;
    println("age:", user.age);  // 0 (default)

    // Direct nil comparison
    if found != nil {
        println("found is not nil");
    }
    if missing == nil {
        println("missing is nil");
    }

    return 0;
}
