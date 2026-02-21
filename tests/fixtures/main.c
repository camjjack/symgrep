#include <stdio.h>

extern int calculate_sum(int a, int b);

int main() {
    int res = calculate_sum(5, 10);

    printf("Result: %d\n", res);

    return 0;
}
