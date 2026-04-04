// Type aliases + HashMap iteration + var keyword
//
// Type aliases create semantic meaning for primitive types.
// Go-style for k, v in map iteration.

type Score = i32;
type Name = str;

struct Player {
    name: i32,
    score: Score,
}

impl Player {
    fn is_winner(self: Player) -> bool {
        return self.score >= 100;
    }
}

fn main() -> i32 {
    // var for mutable variables (Go-style)
    var total_score : Score = 0;

    // HashMap with type aliases
    scores := map_new();
    map_insert(scores, "alice", 120);
    map_insert(scores, "bob", 85);
    map_insert(scores, "carol", 95);

    // Go-style iteration: for key, value in map
    for name, score in scores {
        println(name, "scored", score);
        total_score = total_score + score;
    }

    println("total:", total_score);  // 300

    // GC-managed structs
    p := new Player { name: 1, score: 120 };
    if p.is_winner() {
        println("player is a winner!");
    }

    // f-string interpolation
    println(f"average: {total_score / 3}");

    map_free(scores);
    return 0;
}
