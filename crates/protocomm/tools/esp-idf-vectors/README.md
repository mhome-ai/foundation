# ESP-IDF security 2 vectors

`harness.c` compiles the ESP-IDF `esp_srp.c` (by `#include`) with mbedTLS and prints
`tests/vectors/esp-idf-sec2.json`. The stub headers replace the ESP-IDF logging, error and
random APIs; random bytes are fixed per vector.

```bash
IDF=$HOME/esp/esp-idf/components
cc -O2 -w -Istub -I$IDF/protocomm/src/crypto/srp6a -I$IDF/protocomm/include/crypto/srp6a \
  -I$IDF/mbedtls/mbedtls/include -I$IDF/mbedtls/mbedtls/library \
  harness.c $IDF/protocomm/src/crypto/srp6a/esp_srp_mpi.c $IDF/mbedtls/mbedtls/library/*.c \
  -o harness
./harness > ../../tests/vectors/esp-idf-sec2.json
```

The checked-in vectors come from ESP-IDF v5.5.4 (mbedTLS 3.6.5).
