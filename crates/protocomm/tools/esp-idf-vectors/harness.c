// Builds protocomm security 2 vectors with the ESP-IDF v5.5.4 esp_srp.c and mbedTLS.
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "esp_srp.c"
#include <mbedtls/gcm.h>

static unsigned char g_rand[64];
static size_t g_rand_len;

void esp_fill_random(void *buf, size_t len)
{
    if (len != g_rand_len) {
        fprintf(stderr, "unexpected random length %zu\n", len);
        exit(1);
    }
    memcpy(buf, g_rand, len);
}

static int unhex(const char *hex, unsigned char *out)
{
    int n = strlen(hex) / 2;
    for (int i = 0; i < n; i++) {
        sscanf(hex + 2 * i, "%2hhx", &out[i]);
    }
    return n;
}

static void put_hex(const char *name, const unsigned char *buf, int len, int last)
{
    printf("    \"%s\": \"", name);
    for (int i = 0; i < len; i++) {
        printf("%02x", buf[i]);
    }
    printf("\"%s\n", last ? "" : ",");
}

static void add_one(unsigned char *buf, int len)
{
    for (int i = len - 1; i >= 0; i--) {
        if (++buf[i] != 0) {
            break;
        }
    }
}

/* A = g^a mod N, written as 384 bytes with leading zeros. */
static void client_pub(const unsigned char *a, unsigned char *out384)
{
    esp_mpi_t *n = esp_mpi_new_from_bin(N_3072, sizeof(N_3072));
    esp_mpi_t *g = esp_mpi_new_from_bin(g_3072, sizeof(g_3072));
    esp_mpi_t *am = esp_mpi_new_from_bin((const char *)a, 32);
    esp_mpi_t *A = esp_mpi_new();
    esp_mpi_ctx_t *ctx = esp_mpi_ctx_new();
    esp_mpi_a_exp_b_mod_c(A, g, am, n, ctx);
    mbedtls_mpi_write_binary(A, out384, 384);
    esp_mpi_free(n);
    esp_mpi_free(g);
    esp_mpi_free(am);
    esp_mpi_free(A);
    esp_mpi_ctx_free(ctx);
}

/* Verifier the way esp_srp_gen_salt_verifier derives it: x over the salt as an integer. */
static int verifier(const unsigned char *salt, int salt_len, const char *pass, unsigned char *out)
{
    esp_srp_handle_t *hd = esp_srp_init(ESP_NG_3072);
    esp_mpi_t *s = esp_mpi_new_from_bin((const char *)salt, salt_len);
    int stripped_len = 0;
    char *stripped = esp_mpi_to_bin(s, &stripped_len);
    esp_mpi_t *x = calculate_x(stripped, stripped_len, "meow", 4, pass, strlen(pass));
    esp_mpi_t *v = esp_mpi_new();
    esp_mpi_a_exp_b_mod_c(v, hd->g, x, hd->n, hd->ctx);
    int len = 0;
    char *bytes = esp_mpi_to_bin(v, &len);
    memcpy(out, bytes, len);
    free(bytes);
    free(stripped);
    esp_mpi_free(s);
    esp_mpi_free(x);
    esp_mpi_free(v);
    esp_srp_free(hd);
    return len;
}

static void gcm(const unsigned char *key, const unsigned char *iv, const char *plain,
                unsigned char *out)
{
    mbedtls_gcm_context ctx;
    mbedtls_gcm_init(&ctx);
    mbedtls_gcm_setkey(&ctx, MBEDTLS_CIPHER_ID_AES, key, 256);
    size_t len = strlen(plain);
    mbedtls_gcm_crypt_and_tag(&ctx, MBEDTLS_GCM_ENCRYPT, len, iv, 12, NULL, 0,
                              (const unsigned char *)plain, out, 16, out + len);
    mbedtls_gcm_free(&ctx);
}

static void iv_increment(unsigned char *iv)
{
    add_one(iv + 8, 4);
}

static void vector(const char *name, const char *note, const char *pass, const char *salt_hex,
                   const char *a_hex, const char *b_hex, int find_zero_a, int find_zero_b,
                   int records)
{
    unsigned char salt[16], a[32], b[32];
    int salt_len = unhex(salt_hex, salt);
    unhex(a_hex, a);
    unhex(b_hex, b);

    unsigned char A[384];
    for (;;) {
        client_pub(a, A);
        if (!find_zero_a || A[0] == 0) {
            break;
        }
        add_one(a, 32);
    }

    unsigned char v[384];
    int v_len = verifier(salt, salt_len, pass, v);

    esp_srp_handle_t *hd;
    char *B;
    int B_len;
    for (;;) {
        hd = esp_srp_init(ESP_NG_3072);
        esp_srp_set_salt_verifier(hd, (const char *)salt, salt_len, (const char *)v, v_len);
        memcpy(g_rand, b, 32);
        g_rand_len = 32;
        if (esp_srp_srv_pubkey_from_salt_verifier(hd, &B, &B_len) != ESP_OK) {
            fprintf(stderr, "server pubkey failed\n");
            exit(1);
        }
        if (!find_zero_b || B_len < 384) {
            break;
        }
        esp_srp_free(hd);
        add_one(b, 32);
    }

    char *K;
    uint16_t K_len;
    if (esp_srp_get_session_key(hd, (char *)A, 384, &K, &K_len) != ESP_OK) {
        fprintf(stderr, "session key failed\n");
        exit(1);
    }

    /* M exactly as esp_srp_exchange_proofs computes it before comparing. */
    unsigned char hash_n[64], hash_g[64], hash_xor[64], hash_I[64], M[64];
    mbedtls_sha512((unsigned char *)"meow", 4, hash_I, 0);
    mbedtls_sha512((unsigned char *)hd->bytes_n, hd->len_n, hash_n, 0);
    unsigned char padded_g[384] = {0};
    padded_g[383] = 5;
    mbedtls_sha512(padded_g, 384, hash_g, 0);
    for (int i = 0; i < 64; i++) {
        hash_xor[i] = hash_n[i] ^ hash_g[i];
    }
    mbedtls_sha512_context sha;
    mbedtls_sha512_init(&sha);
    mbedtls_sha512_starts(&sha, 0);
    mbedtls_sha512_update(&sha, hash_xor, 64);
    mbedtls_sha512_update(&sha, hash_I, 64);
    mbedtls_sha512_update(&sha, (unsigned char *)hd->bytes_s, hd->len_s);
    mbedtls_sha512_update(&sha, (unsigned char *)hd->bytes_A, hd->len_A);
    mbedtls_sha512_update(&sha, (unsigned char *)hd->bytes_B, hd->len_B);
    mbedtls_sha512_update(&sha, (unsigned char *)K, 64);
    mbedtls_sha512_finish(&sha, M);
    mbedtls_sha512_free(&sha);

    char HAMK[64];
    if (esp_srp_exchange_proofs(hd, "meow", 4, (char *)M, HAMK) != ESP_OK) {
        fprintf(stderr, "esp_srp rejected the proof\n");
        exit(1);
    }

    unsigned char wrong[64];
    memcpy(wrong, M, 64);
    wrong[0] ^= 1;
    char ignored[64];
    if (esp_srp_exchange_proofs(hd, "meow", 4, (char *)wrong, ignored) == ESP_OK) {
        fprintf(stderr, "esp_srp accepted a wrong proof\n");
        exit(1);
    }

    printf("  \"%s\": {\n", name);
    printf("    \"note\": \"%s\",\n", note);
    printf("    \"username\": \"meow\",\n");
    printf("    \"password\": \"%s\",\n", pass);
    put_hex("salt", salt, salt_len, 0);
    put_hex("verifier", v, v_len, 0);
    put_hex("a", a, 32, 0);
    put_hex("b", b, 32, 0);
    put_hex("A", A, 384, 0);
    put_hex("B", (unsigned char *)B, B_len, 0);
    put_hex("M", M, 64, 0);
    put_hex("HAMK", (unsigned char *)HAMK, 64, 0);
    put_hex("K", (unsigned char *)K, 64, !records);
    if (records) {
        unsigned char iv[12] = {0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0, 0, 0, 1};
        put_hex("deviceNonce", iv, 12, 0);
        const char *steps[][2] = {
            {"client", "{\"op\":\"info\"}"},
            {"device", "{\"protocol\":1}"},
            {"client", "{\"op\":\"status\"}"},
            {"device", "{\"state\":\"secured\"}"},
        };
        printf("    \"records\": [\n");
        for (int i = 0; i < 4; i++) {
            unsigned char out[128];
            gcm((unsigned char *)K, iv, steps[i][1], out);
            printf("      { \"from\": \"%s\", \"counter\": %d, \"plaintext\": \"", steps[i][0],
                   i + 1);
            for (const char *p = steps[i][1]; *p; p++) {
                printf(*p == '"' ? "\\\"" : "%c", *p);
            }
            printf("\", \"record\": \"");
            for (size_t j = 0; j < strlen(steps[i][1]) + 16; j++) {
                printf("%02x", out[j]);
            }
            printf("\" }%s\n", i == 3 ? "" : ",");
            iv_increment(iv);
        }
        printf("    ],\n");
        /* After a lost exchange at counter 5 the client has advanced to 6; the device is at 5. */
        const char *retry = "{\"op\":\"status\"}";
        unsigned char lost[128], stale[128];
        gcm((unsigned char *)K, iv, retry, lost);
        iv_increment(iv);
        gcm((unsigned char *)K, iv, retry, stale);
        printf("    \"afterFailure\": {\n");
        printf("      \"plaintext\": \"{\\\"op\\\":\\\"status\\\"}\",\n");
        printf("      \"lostCounter\": 5,\n");
        put_hex("lostRecord", lost, strlen(retry) + 16, 0);
        printf("      \"staleCounter\": 6,\n");
        put_hex("staleRecord", stale, strlen(retry) + 16, 1);
        printf("    }\n");
    }
    printf("  }");
    esp_srp_free(hd);
}

int main(void)
{
    const char *a = "8222222222222222222222222222222222222222222222222222222222222222";
    const char *b = "9111111111111111111111111111111111111111111111111111111111111111";
    printf("{\n");
    printf("  \"source\": \"ESP-IDF v5.5.4 components/protocomm esp_srp.c with mbedTLS 3.6.5\",\n");
    vector("basic", "full-length salt, A and B; records use the security 2 patch 1 nonce counter",
           "042137", "5ea1b2c3d4e5f60718293a4b5c6d7e8f", a, b, 0, 0, 1);
    printf(",\n");
    vector("leadingZeroSalt",
           "16-byte salt starting with 0x00: x uses the salt as an integer, M the salt bytes as sent",
           "042137", "00a1b2c3d4e5f60718293a4b5c6d7e8f", a, b, 0, 0, 0);
    printf(",\n");
    vector("leadingZeroA",
           "g^a has a leading zero byte; ESP-IDF requires A as exactly 384 bytes, so clients pick a new a",
           "042137", "5ea1b2c3d4e5f60718293a4b5c6d7e8f", a, b, 1, 0, 0);
    printf(",\n");
    vector("leadingZeroB",
           "B has a leading zero byte; the device sends it without that byte and M uses it as sent",
           "042137", "5ea1b2c3d4e5f60718293a4b5c6d7e8f", a, b, 0, 1, 0);
    printf("\n}\n");
    return 0;
}
