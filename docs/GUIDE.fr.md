# Guide pas à pas : récupérer ses fichiers sur un disque de PS3 formaté

*[English version](GUIDE.md)*

Ce guide s'adresse aux débutants. Il n'est pas nécessaire de savoir programmer, mais il faudra taper quelques commandes : chacune est donnée telle quelle.

Comptez une demi-journée, surtout parce que la copie du disque prend du temps.

## Avant de commencer

**Arrêtez d'utiliser la PS3.** Un formatage n'efface pas vraiment les données, mais chaque nouvelle sauvegarde, installation ou mise à jour peut écraser les fichiers que vous voulez retrouver.

Il vous faut :

- **La PS3 d'origine.** La clé de chiffrement est propre à chaque console. Un disque ne peut être lu qu'avec la clé de la console dans laquelle il était.
- **Un adaptateur USB vers SATA 2,5 pouces** (une dizaine d'euros), pour brancher le disque de la PS3 sur votre ordinateur.
- **Un second disque dur 2,5 pouces SATA**, vide, qui restera dans la PS3 pendant les manipulations, pour ne jamais toucher au disque d'origine.
- **Une clé USB** au format FAT32.
- **De la place sur l'ordinateur** : au moins **deux fois la taille du disque de la PS3** (une copie chiffrée et une copie déchiffrée). Un disque externe fait très bien l'affaire.

Notez le **numéro de modèle** de votre PS3, inscrit au dos ou dessous (« CECH-… »). Il sert à l'étape 3.

## Étape 1 : sortir le disque de la PS3

1. Éteignez la PS3 et débranchez-la.
2. Retirez le disque dur. L'emplacement dépend du modèle : trappe sur le côté des PS3 « Fat », trappe en façade ou sur le côté des « Slim » et « Super Slim ». Les guides d'iFixit montrent la démarche pour chaque modèle.
3. Rangez ce disque à l'abri. **On ne le remettra pas dans la PS3** avant la fin de la récupération.

## Étape 2 : faire une copie exacte du disque

On copie le disque entier, octet par octet, dans un fichier `ps3_image.img`. Tout le reste se fait ensuite sur cette copie, et le disque d'origine n'est plus touché.

Branchez le disque sur l'ordinateur avec l'adaptateur USB-SATA.

> ⚠️ Votre ordinateur va dire que le disque est illisible et proposer de l'**initialiser** ou de le **formater**. **Refusez toujours** (cliquez sur « Ignorer » ou « Annuler »). Accepter effacerait le disque.

### Sur Mac

1. Ouvrez l'application **Terminal** (Applications > Utilitaires).
2. Tapez la commande suivante pour lister les disques, puis appuyez sur Entrée :
   ```bash
   diskutil list
   ```
   Repérez le disque de la PS3 grâce à sa taille (par exemple `*500.1 GB`) et notez son nom, par exemple `/dev/disk4`. **Vérifiez bien : se tromper de disque peut écraser vos données.**
3. Lancez la copie, en remplaçant `disk4` par le bon numéro. Ajoutez un `r` devant (`rdisk4`), ce qui rend la copie bien plus rapide :
   ```bash
   sudo dd if=/dev/rdisk4 of=ps3_image.img bs=1m conv=noerror,sync
   ```
   Le Mac demande votre mot de passe : tapez-le, même si rien ne s'affiche, puis appuyez sur Entrée.
4. La copie dure de une à plusieurs heures. Appuyez sur **Ctrl + T** pour voir où elle en est. Elle est terminée quand l'invite de commande réapparaît.

### Sous Windows

Windows n'a pas d'outil de copie brute intégré. Utilisez **HDD Raw Copy Tool** (gratuit, sur hddguru.com) :

1. Lancez-le en tant qu'administrateur et choisissez le disque de la PS3 comme **source**. Repérez-le à sa taille.
2. Choisissez **FILE** comme destination et enregistrez au format brut **.img** (pas la version compressée `.imgc`).
3. Lancez la copie.

### Sous Linux

```bash
lsblk                     # repérez le disque de la PS3, par exemple /dev/sdb
sudo dd if=/dev/sdb of=ps3_image.img bs=1M status=progress conv=noerror,sync
```

Une fois la copie terminée, débranchez le disque d'origine et rangez-le.

## Étape 3 : récupérer la clé de la console (`eid_root_key`)

La clé est stockée dans la PS3 elle-même. Pour la lire, la console doit fonctionner sous un **firmware personnalisé (CFW)**.

> ⚠️ **Limites importantes**
> - **Toutes les PS3 ne peuvent pas installer un CFW.** Seules certaines PS3 « Fat » et « Slim » le peuvent. Les **Super Slim (CECH-4xxx)** et les Slim récentes ne le peuvent pas. Sur ces consoles, il n'existe aujourd'hui aucun moyen connu de récupérer la clé, et donc de déchiffrer le disque.
> - **HEN ne suffit pas.** Le « Homebrew Enabler » ne permet pas d'extraire la clé, et des utilisateurs signalent qu'essayer peut empêcher la console de démarrer.
> - Installer un CFW comporte un **risque de rendre la console inutilisable** (« brick ») si les instructions ne sont pas suivies à la lettre. Vous le faites à vos risques.
> - Ne vous connectez pas au PlayStation Network tant que le CFW est installé.

1. **Mettez le second disque (vide) dans la PS3.** Au démarrage, la console demandera de réinstaller le logiciel système depuis une clé USB, puis formatera ce disque : c'est normal, puisque c'est le disque vide. Le guide de l'étape suivante explique quel fichier mettre sur la clé. Le disque d'origine, lui, reste rangé.
2. **Vérifiez que votre console accepte un CFW** avec l'outil **MinVerChk** (voir la page « PS3:MinVerChk » du wiki ConsoleMods). Il affiche « Update data of version ____ or later can be installed on this system ». **Si le nombre est 3.56 ou moins, la console est compatible.** Sinon, arrêtez-vous ici.
3. **Installez un CFW** en suivant pas à pas un guide reconnu, comme « PS3:How to Hack your PS3 » sur le wiki ConsoleMods. Ce guide ne détaille pas cette étape, qui change selon les modèles et les versions.
4. **Installez Rebug Toolbox** (un fichier `.pkg`) depuis une clé USB, avec **Install Package Files** dans le menu Jeu de la PS3.
5. Ouvrez **Rebug Toolbox**, allez dans l'onglet **Utilities** et choisissez **Dump eid root key**. La console redémarre.
6. La clé est enregistrée dans `/dev_hdd0/game/RBGTLBOX2/USRDIR/eid_root_key`. Copiez ce fichier sur la clé USB avec un gestionnaire de fichiers comme **multiMAN**, ou par FTP.
7. Sur l'ordinateur, vérifiez que le fichier `eid_root_key` fait exactement **48 octets**.

> 🔒 Cette clé est unique à votre console. **Ne la partagez jamais** et ne la publiez nulle part.

## Étape 4 : installer ps3recover

Téléchargez le programme correspondant à votre ordinateur sur la page [Releases](https://github.com/crocdeine/ps3-data-recovery/releases) du projet :

| Ordinateur | Fichier |
|---|---|
| Mac récent (puce M1, M2…) | `ps3recover-macos-apple-silicon.tar.gz` |
| Mac Intel | `ps3recover-macos-intel.tar.gz` |
| Windows | `ps3recover-windows-x86_64.zip` |
| Linux | `ps3recover-linux-x86_64.tar.gz` |

Décompressez-le et placez `ps3recover` (ou `ps3recover.exe`) dans le même dossier que `ps3_image.img` et `eid_root_key`.

Ouvrez un terminal dans ce dossier :

- **Mac** : dans le Terminal, tapez `cd ` (avec une espace), glissez le dossier dans la fenêtre, puis appuyez sur Entrée. Le Mac bloque les programmes téléchargés hors de l'App Store ; débloquez celui-ci une fois avec :
  ```bash
  xattr -d com.apple.quarantine ps3recover
  ```
- **Windows** : ouvrez le dossier dans l'Explorateur, faites un clic droit dans un espace vide et choisissez « Ouvrir dans le Terminal ». Dans les commandes ci-dessous, remplacez `./ps3recover` par `.\ps3recover.exe`.

## Étape 5 : déchiffrer le disque

```bash
./ps3recover decrypt ps3_image.img ps3_decrypted.img --key eid_root_key
```

- Si la clé est la bonne, le programme affiche `Key accepted: drive detected as …` puis la progression.
- Si la clé ne correspond pas au disque, il s'arrête aussitôt avec un message d'erreur et n'écrit rien. Vérifiez que la clé vient bien de la console d'où sort le disque.

## Étape 6 : retrouver les fichiers

```bash
./ps3recover recover ps3_decrypted.img fichiers_retrouves
```

Le programme parcourt tout le disque déchiffré et range ce qu'il trouve dans le dossier `fichiers_retrouves` :

| Dossier | Contenu |
|---|---|
| `photos` | Photos et captures (jpg, png, gif…) |
| `videos` | Vidéos (mp4, enregistrements PS3 en m2ts…) |
| `music` | Musique (mp3, wav…) |
| `ps3_files` | Fichiers PS3 : informations de sauvegarde (`.sfo`), jeux et mises à jour (`.pkg`)… |
| `documents`, `archives` | Fichiers texte et archives (zip…) |

Dans chaque dossier, un sous-dossier `corrupted` contient les fichiers abîmés, souvent parce qu'ils étaient éparpillés en plusieurs morceaux sur le disque. Pour que les vidéos et la musique soient aussi vérifiées, installez [ffmpeg](https://ffmpeg.org/) avant de lancer la commande.

Les fichiers portent un nom en chiffres hexadécimaux, qui correspond à leur position sur le disque. Les noms d'origine ne peuvent pas être retrouvés, car le formatage les a effacés.

## Ce qu'il faut savoir

- **Rien n'est garanti.** Ce qui a été écrasé depuis le formatage est perdu.
- **Les sauvegardes de jeux** sont plus difficiles à retrouver que les photos et les vidéos : elles sont faites de plusieurs fichiers, souvent chiffrés par le jeu lui-même. L'outil repère leurs fichiers `PARAM.SFO`, ce qui indique au moins de quels jeux il s'agit.
- **Un problème ?** Ouvrez une [issue](https://github.com/crocdeine/ps3-data-recovery/issues) en indiquant votre modèle de PS3 et le message affiché, **sans jamais joindre votre clé**.
