#include <stdio.h>
#include <stdlib.h>
#include <time.h>

typedef struct Node { struct Node *left, *right; } Node;

Node *make_tree(int depth) {
    Node *n = (Node *)malloc(sizeof(Node));
    if (depth > 0) { n->left = make_tree(depth-1); n->right = make_tree(depth-1); }
    else { n->left = n->right = NULL; }
    return n;
}

int check_tree(Node *n) {
    if (!n->left) return 1;
    return 1 + check_tree(n->left) + check_tree(n->right);
}

void free_tree(Node *n) {
    if (n->left) { free_tree(n->left); free_tree(n->right); }
    free(n);
}

int main() {
    int max_depth = 18;
    struct timespec s, e;
    clock_gettime(CLOCK_MONOTONIC, &s);

    Node *stretch = make_tree(max_depth + 1);
    printf("stretch tree of depth %d, check: %d\n", max_depth+1, check_tree(stretch));
    free_tree(stretch);

    Node *long_lived = make_tree(max_depth);
    for (int depth = 4; depth <= max_depth; depth += 2) {
        int iterations = 1;
        for (int i = 0; i < max_depth - depth; i++) iterations *= 2;
        int check = 0;
        for (int i = 0; i < iterations; i++) {
            Node *t = make_tree(depth);
            check += check_tree(t);
            free_tree(t);
        }
        printf("%d trees of depth %d, check: %d\n", iterations, depth, check);
    }
    printf("long lived tree of depth %d, check: %d\n", max_depth, check_tree(long_lived));
    free_tree(long_lived);

    clock_gettime(CLOCK_MONOTONIC, &e);
    long ms = (e.tv_sec-s.tv_sec)*1000 + (e.tv_nsec-s.tv_nsec)/1000000;
    printf("binary-trees (depth %d): %ldms\n", max_depth, ms);
    return 0;
}
