#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

#define AES_BLOCK_SIZE 16

// Prototype des fonctions
void print_progress(size_t decrypted, size_t total);

int main(int argc, char *argv[]) {
    if (argc != 4) {
        fprintf(stderr, "Usage: %s <image chiffrée> <image déchiffrée> <eid_root_key>\n", argv[0]);
        return 1;
    }

    FILE *image_file = fopen(argv[1], "rb");
    if (!image_file) {
        perror("Erreur lors de l'ouverture de l'image");
        return 1;
    }

    // Lecture de l'image pour obtenir sa taille
    fseek(image_file, 0, SEEK_END);
    size_t image_size = ftell(image_file);
    fseek(image_file, 0, SEEK_SET);

    // Ouverture du fichier pour la clé eid_root_key
    FILE *keyfile = fopen(argv[3], "rb");
    if (!keyfile) {
        perror("Erreur lors de l'ouverture du fichier eid_root_key");
        fclose(image_file);
        return 1;
    }

    // Lecture de la clé eid_root_key
    uint8_t eid_root_key[48];
    if (fread(eid_root_key, 1, 48, keyfile) != 48) {
        fprintf(stderr, "Erreur de lecture de la eid_root_key : taille lue incorrecte\n");
        fclose(image_file);
        fclose(keyfile);
        return 1;
    }
    fclose(keyfile);

    printf("eid_root_key chargée :\n");
    for (int i = 0; i < 48; i++) {
        printf("%02x", eid_root_key[i]);
        if ((i + 1) % 16 == 0) printf("\n");
        else printf(" ");
    }
    printf("\n");

    // Ouverture du fichier de sortie
    FILE *output_file = fopen(argv[2], "wb");
    if (!output_file) {
        perror("Erreur lors de l'ouverture de l'image déchiffrée");
        fclose(image_file);
        return 1;
    }

    // Processus de déchiffrement (à ajuster selon le mode AES spécifique)
    size_t decrypted = 0;
    uint8_t buffer[AES_BLOCK_SIZE];

    while (decrypted < image_size) {
        // Lire un bloc de l'image
        size_t block_size = fread(buffer, 1, AES_BLOCK_SIZE, image_file);
        if (block_size == 0) break;

        // Ici, déchiffrement des données avec la clé eid_root_key (à ajuster selon le mode AES)

        // Exemple de déchiffrement fictif
        for (size_t i = 0; i < block_size; i++) {
            buffer[i] ^= eid_root_key[i % 48];  // Simple XOR pour illustration
        }

        // Écrire le bloc déchiffré
        fwrite(buffer, 1, block_size, output_file);
        decrypted += block_size;

        // Afficher la progression
        print_progress(decrypted, image_size);
    }

    printf("\nDéchiffrement terminé.\n");
    fclose(image_file);
    fclose(output_file);
    return 0;
}

// Affichage de la progression
void print_progress(size_t decrypted, size_t total) {
    int percent = (int)((double)decrypted / total * 100);
    printf("\rDéchiffré: %d MB [%3d%%]", decrypted / (1024 * 1024), percent);
    fflush(stdout);
}
