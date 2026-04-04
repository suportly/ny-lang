package main

import (
	"fmt"
	"time"
)

type Node struct{ left, right *Node }

func makeTree(depth int) *Node {
	n := &Node{}
	if depth > 0 { n.left = makeTree(depth - 1); n.right = makeTree(depth - 1) }
	return n
}

func checkTree(n *Node) int {
	if n.left == nil { return 1 }
	return 1 + checkTree(n.left) + checkTree(n.right)
}

func main() {
	maxDepth := 18
	start := time.Now()
	stretch := makeTree(maxDepth + 1)
	fmt.Printf("stretch tree of depth %d, check: %d\n", maxDepth+1, checkTree(stretch))
	longLived := makeTree(maxDepth)
	for depth := 4; depth <= maxDepth; depth += 2 {
		iterations := 1
		for i := 0; i < maxDepth-depth; i++ { iterations *= 2 }
		check := 0
		for i := 0; i < iterations; i++ {
			t := makeTree(depth)
			check += checkTree(t)
		}
		fmt.Printf("%d trees of depth %d, check: %d\n", iterations, depth, check)
	}
	fmt.Printf("long lived tree of depth %d, check: %d\n", maxDepth, checkTree(longLived))
	elapsed := time.Since(start)
	fmt.Printf("binary-trees (depth %d): %dms\n", maxDepth, elapsed.Milliseconds())
}
