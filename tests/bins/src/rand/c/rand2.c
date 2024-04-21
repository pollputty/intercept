#include <time.h>
#include <stdlib.h>
#include <stdio.h>

int main() {
    srand(time(NULL));   // Initialization, should only be called once.

    int r = rand();

    printf("%d\n", r);
    return 0;
}