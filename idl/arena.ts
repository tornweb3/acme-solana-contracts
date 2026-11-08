/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/arena.json`.
 */
export type Arena = {
  "address": "G8hRPxkadXhZcuyeH7vWCqmfAqfWNAuxJnBftPezcUdK",
  "metadata": {
    "name": "arena",
    "version": "0.2.0",
    "spec": "0.1.0",
    "description": "ACME arena: escrow for live-race rooms and prize cups"
  },
  "instructions": [
    {
      "name": "acceptAdmin",
      "discriminator": [
        112,
        42,
        45,
        90,
        116,
        181,
        13,
        170
      ],
      "accounts": [
        {
          "name": "newAdmin",
          "signer": true
        },
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "addMint",
      "discriminator": [
        171,
        222,
        111,
        37,
        60,
        166,
        208,
        108
      ],
      "accounts": [
        {
          "name": "admin",
          "writable": true,
          "signer": true,
          "relations": [
            "config"
          ]
        },
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "allowedMint",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  109,
                  105,
                  110,
                  116
                ]
              },
              {
                "kind": "arg",
                "path": "mintKey"
              }
            ]
          }
        },
        {
          "name": "mint",
          "docs": [
            "Absent when `mint_key` is NATIVE_SOL."
          ],
          "optional": true
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "mintKey",
          "type": "pubkey"
        },
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "mintParams"
            }
          }
        }
      ]
    },
    {
      "name": "addRuleset",
      "discriminator": [
        188,
        93,
        62,
        242,
        1,
        46,
        216,
        211
      ],
      "accounts": [
        {
          "name": "admin",
          "writable": true,
          "signer": true,
          "relations": [
            "config"
          ]
        },
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "ruleset",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  117,
                  108,
                  101,
                  115,
                  101,
                  116
                ]
              },
              {
                "kind": "arg",
                "path": "id"
              }
            ]
          }
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "id",
          "type": "u32"
        },
        {
          "name": "hash",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "ratingPoints",
          "type": {
            "array": [
              "u8",
              6
            ]
          }
        }
      ]
    },
    {
      "name": "applyRoles",
      "discriminator": [
        178,
        48,
        223,
        26,
        178,
        139,
        7,
        185
      ],
      "accounts": [
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "cancelCup",
      "discriminator": [
        140,
        5,
        209,
        151,
        24,
        152,
        90,
        109
      ],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "cup",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  117,
                  112
                ]
              },
              {
                "kind": "account",
                "path": "cup.id",
                "account": "cup"
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "cancelRoom",
      "discriminator": [
        91,
        107,
        215,
        178,
        200,
        224,
        241,
        237
      ],
      "accounts": [
        {
          "name": "creator",
          "signer": true
        },
        {
          "name": "payout",
          "accounts": [
            {
              "name": "room",
              "writable": true
            },
            {
              "name": "mint",
              "optional": true
            },
            {
              "name": "escrowTokens",
              "writable": true,
              "optional": true
            },
            {
              "name": "tokenProgram",
              "optional": true
            }
          ]
        }
      ],
      "args": []
    },
    {
      "name": "claimPrize",
      "discriminator": [
        157,
        233,
        139,
        121,
        246,
        62,
        234,
        235
      ],
      "accounts": [
        {
          "name": "claimer",
          "docs": [
            "The winner (until `claim_until`), or anyone pushing it after."
          ],
          "signer": true
        },
        {
          "name": "vault",
          "accounts": [
            {
              "name": "cup",
              "writable": true,
              "pda": {
                "seeds": [
                  {
                    "kind": "const",
                    "value": [
                      99,
                      117,
                      112
                    ]
                  },
                  {
                    "kind": "account",
                    "path": "cup.id",
                    "account": "cup"
                  }
                ]
              }
            },
            {
              "name": "mint",
              "optional": true
            },
            {
              "name": "escrowTokens",
              "writable": true,
              "optional": true
            },
            {
              "name": "tokenProgram",
              "optional": true
            }
          ]
        },
        {
          "name": "entry",
          "docs": [
            "Entries cups: the winner's entry (it must not have been refunded)."
          ],
          "optional": true
        },
        {
          "name": "destination",
          "docs": [
            "(SPL); checked when paid."
          ],
          "writable": true
        }
      ],
      "args": [
        {
          "name": "rank",
          "type": "u16"
        },
        {
          "name": "wallet",
          "type": "pubkey"
        },
        {
          "name": "proof",
          "type": {
            "vec": {
              "array": [
                "u8",
                32
              ]
            }
          }
        }
      ]
    },
    {
      "name": "closeCup",
      "discriminator": [
        57,
        179,
        108,
        22,
        163,
        67,
        252,
        155
      ],
      "accounts": [
        {
          "name": "cup",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  117,
                  112
                ]
              },
              {
                "kind": "account",
                "path": "cup.id",
                "account": "cup"
              }
            ]
          }
        },
        {
          "name": "rentPayer",
          "writable": true
        },
        {
          "name": "sponsorDestination",
          "writable": true
        },
        {
          "name": "hostDestination",
          "writable": true,
          "optional": true
        },
        {
          "name": "treasuryDestination",
          "writable": true
        },
        {
          "name": "prizeVault",
          "writable": true
        },
        {
          "name": "prizeVaultTokens",
          "docs": [
            "what the cup owed (needed only when there are some); checked when paid."
          ],
          "writable": true,
          "optional": true
        },
        {
          "name": "mint",
          "docs": [
            "Writable: closing a transfer-fee token's escrow harvests its withheld fees into the mint."
          ],
          "writable": true,
          "optional": true
        },
        {
          "name": "escrowTokens",
          "writable": true,
          "optional": true
        },
        {
          "name": "tokenProgram",
          "optional": true
        }
      ],
      "args": []
    },
    {
      "name": "closeEntry",
      "discriminator": [
        132,
        26,
        202,
        145,
        190,
        37,
        114,
        67
      ],
      "accounts": [
        {
          "name": "entry",
          "writable": true
        },
        {
          "name": "rentPayer",
          "writable": true
        },
        {
          "name": "cup"
        }
      ],
      "args": []
    },
    {
      "name": "closeRoom",
      "discriminator": [
        152,
        197,
        88,
        192,
        98,
        197,
        51,
        211
      ],
      "accounts": [
        {
          "name": "room",
          "writable": true
        },
        {
          "name": "rentPayer",
          "writable": true
        },
        {
          "name": "prizeVault",
          "writable": true
        },
        {
          "name": "prizeVaultTokens",
          "docs": [
            "tokens left in the escrow (needed only when there are some); checked when paid."
          ],
          "writable": true,
          "optional": true
        },
        {
          "name": "mint",
          "docs": [
            "Writable: closing a transfer-fee token's escrow harvests its withheld fees into the mint."
          ],
          "writable": true,
          "optional": true
        },
        {
          "name": "escrowTokens",
          "writable": true,
          "optional": true
        },
        {
          "name": "tokenProgram",
          "optional": true
        }
      ],
      "args": []
    },
    {
      "name": "createCup",
      "discriminator": [
        111,
        24,
        131,
        0,
        59,
        231,
        160,
        88
      ],
      "accounts": [
        {
          "name": "organizer",
          "docs": [
            "Organizer and sponsor: pays the guarantee (if any)."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "payer",
          "docs": [
            "Pays the cup's rent (the organizer, or the house keeper)."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "ruleset"
        },
        {
          "name": "allowedMint",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  109,
                  105,
                  110,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "allowedMint.mint",
                "account": "allowedMint"
              }
            ]
          }
        },
        {
          "name": "cup",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  117,
                  112
                ]
              },
              {
                "kind": "arg",
                "path": "id"
              }
            ]
          }
        },
        {
          "name": "host",
          "docs": [
            "An admin-approved host's record (the path with no token)."
          ],
          "optional": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  115,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "organizer"
              }
            ]
          }
        },
        {
          "name": "holderTokens",
          "docs": [
            "A token-holding host's project-token account."
          ],
          "optional": true
        },
        {
          "name": "mint",
          "optional": true
        },
        {
          "name": "escrowTokens",
          "docs": [
            "The cup's associated token account; create it earlier in the transaction."
          ],
          "writable": true,
          "optional": true
        },
        {
          "name": "organizerTokens",
          "writable": true,
          "optional": true
        },
        {
          "name": "tokenProgram",
          "optional": true
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "id",
          "type": "u64"
        },
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "cupParams"
            }
          }
        }
      ]
    },
    {
      "name": "createRoom",
      "discriminator": [
        130,
        166,
        32,
        2,
        247,
        120,
        178,
        53
      ],
      "accounts": [
        {
          "name": "creator",
          "docs": [
            "Stakes, and takes the first seat."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "payer",
          "docs": [
            "Pays the room's rent (the race server's sponsor, or the creator); it gets",
            "it back when the room closes."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "gate",
          "signer": true,
          "optional": true
        },
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "ruleset"
        },
        {
          "name": "allowedMint",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  109,
                  105,
                  110,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "allowedMint.mint",
                "account": "allowedMint"
              }
            ]
          }
        },
        {
          "name": "room",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  111,
                  111,
                  109
                ]
              },
              {
                "kind": "arg",
                "path": "code"
              }
            ]
          }
        },
        {
          "name": "upgrades",
          "docs": [
            "house program (may not exist yet: stock). Checked in the handler."
          ],
          "optional": true
        },
        {
          "name": "holderTokens",
          "docs": [
            "The creator's project-token account, to claim the holder fee for the room."
          ],
          "optional": true
        },
        {
          "name": "mint",
          "docs": [
            "Token rooms only (all four absent for SOL). The escrow is the room's",
            "associated token account; create it (idempotently) earlier in the same",
            "transaction."
          ],
          "optional": true
        },
        {
          "name": "escrowTokens",
          "writable": true,
          "optional": true
        },
        {
          "name": "creatorTokens",
          "writable": true,
          "optional": true
        },
        {
          "name": "tokenProgram",
          "optional": true
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "code",
          "type": {
            "array": [
              "u8",
              8
            ]
          }
        },
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "roomParams"
            }
          }
        }
      ]
    },
    {
      "name": "drawSeed",
      "discriminator": [
        190,
        124,
        125,
        7,
        74,
        109,
        102,
        159
      ],
      "accounts": [
        {
          "name": "room",
          "writable": true
        },
        {
          "name": "slotHashes",
          "address": "SysvarS1otHashes111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "enterCup",
      "discriminator": [
        61,
        26,
        0,
        32,
        103,
        14,
        108,
        196
      ],
      "accounts": [
        {
          "name": "player",
          "writable": true,
          "signer": true
        },
        {
          "name": "payer",
          "docs": [
            "Pays the entry's rent (the race server's sponsor, or the player)."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "gate",
          "signer": true,
          "optional": true
        },
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "cup",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  117,
                  112
                ]
              },
              {
                "kind": "account",
                "path": "cup.id",
                "account": "cup"
              }
            ]
          }
        },
        {
          "name": "entry",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  110,
                  116,
                  114,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "cup"
              },
              {
                "kind": "account",
                "path": "player"
              }
            ]
          }
        },
        {
          "name": "mint",
          "optional": true
        },
        {
          "name": "escrowTokens",
          "writable": true,
          "optional": true
        },
        {
          "name": "playerTokens",
          "writable": true,
          "optional": true
        },
        {
          "name": "tokenProgram",
          "optional": true
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "finalizeCup",
      "discriminator": [
        179,
        255,
        227,
        77,
        123,
        81,
        221,
        121
      ],
      "accounts": [
        {
          "name": "cup",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  117,
                  112
                ]
              },
              {
                "kind": "account",
                "path": "cup.id",
                "account": "cup"
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "initialize",
      "discriminator": [
        175,
        175,
        109,
        31,
        13,
        152,
        155,
        237
      ],
      "accounts": [
        {
          "name": "admin",
          "writable": true,
          "signer": true
        },
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "program",
          "docs": [
            "The program itself, used to reach its ProgramData record."
          ],
          "address": "G8hRPxkadXhZcuyeH7vWCqmfAqfWNAuxJnBftPezcUdK"
        },
        {
          "name": "programData",
          "docs": [
            "Only the upgrade authority may bootstrap the protocol. Without this the",
            "first caller after deploy would become admin and pick the treasury."
          ]
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "configParams"
            }
          }
        },
        {
          "name": "resolvers",
          "type": {
            "vec": "pubkey"
          }
        }
      ]
    },
    {
      "name": "joinRoom",
      "discriminator": [
        95,
        232,
        188,
        81,
        124,
        130,
        78,
        139
      ],
      "accounts": [
        {
          "name": "player",
          "writable": true,
          "signer": true
        },
        {
          "name": "gate",
          "signer": true,
          "optional": true
        },
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "room",
          "writable": true
        },
        {
          "name": "ruleset"
        },
        {
          "name": "upgrades",
          "optional": true
        },
        {
          "name": "mint",
          "optional": true
        },
        {
          "name": "escrowTokens",
          "writable": true,
          "optional": true
        },
        {
          "name": "playerTokens",
          "writable": true,
          "optional": true
        },
        {
          "name": "tokenProgram",
          "optional": true
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "expectedStake",
          "type": "u64"
        }
      ]
    },
    {
      "name": "leaveRoom",
      "discriminator": [
        249,
        42,
        239,
        128,
        192,
        20,
        114,
        156
      ],
      "accounts": [
        {
          "name": "player",
          "writable": true,
          "signer": true
        },
        {
          "name": "room",
          "writable": true
        },
        {
          "name": "mint",
          "optional": true
        },
        {
          "name": "escrowTokens",
          "writable": true,
          "optional": true
        },
        {
          "name": "playerTokens",
          "docs": [
            "Token rooms: where the stake goes back to (owned by the player)."
          ],
          "writable": true,
          "optional": true
        },
        {
          "name": "tokenProgram",
          "optional": true
        }
      ],
      "args": []
    },
    {
      "name": "lockRoom",
      "discriminator": [
        97,
        147,
        184,
        7,
        215,
        44,
        200,
        216
      ],
      "accounts": [
        {
          "name": "resolver",
          "signer": true
        },
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "room",
          "writable": true
        }
      ],
      "args": []
    },
    {
      "name": "postResults",
      "discriminator": [
        37,
        143,
        165,
        109,
        217,
        84,
        94,
        27
      ],
      "accounts": [
        {
          "name": "resolver",
          "signer": true
        },
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "cup",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  117,
                  112
                ]
              },
              {
                "kind": "account",
                "path": "cup.id",
                "account": "cup"
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "resultsRoot",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "scorers",
          "type": "u32"
        },
        {
          "name": "places",
          "type": "u8"
        },
        {
          "name": "standingsHash",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "resultsUri",
          "type": "string"
        }
      ]
    },
    {
      "name": "proposeAdmin",
      "discriminator": [
        121,
        214,
        199,
        212,
        87,
        39,
        117,
        234
      ],
      "accounts": [
        {
          "name": "admin",
          "signer": true,
          "relations": [
            "config"
          ]
        },
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "newAdmin",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "proposeRoles",
      "discriminator": [
        175,
        43,
        80,
        85,
        231,
        56,
        200,
        39
      ],
      "accounts": [
        {
          "name": "admin",
          "signer": true,
          "relations": [
            "config"
          ]
        },
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "guardian",
          "type": "pubkey"
        },
        {
          "name": "gate",
          "type": "pubkey"
        },
        {
          "name": "resolvers",
          "type": {
            "vec": "pubkey"
          }
        }
      ]
    },
    {
      "name": "refundEntry",
      "discriminator": [
        214,
        5,
        136,
        23,
        253,
        7,
        230,
        81
      ],
      "accounts": [
        {
          "name": "vault",
          "accounts": [
            {
              "name": "cup",
              "writable": true,
              "pda": {
                "seeds": [
                  {
                    "kind": "const",
                    "value": [
                      99,
                      117,
                      112
                    ]
                  },
                  {
                    "kind": "account",
                    "path": "cup.id",
                    "account": "cup"
                  }
                ]
              }
            },
            {
              "name": "mint",
              "optional": true
            },
            {
              "name": "escrowTokens",
              "writable": true,
              "optional": true
            },
            {
              "name": "tokenProgram",
              "optional": true
            }
          ]
        },
        {
          "name": "entry",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  101,
                  110,
                  116,
                  114,
                  121
                ]
              },
              {
                "kind": "account",
                "path": "vault.cup",
                "account": "cupVault"
              },
              {
                "kind": "account",
                "path": "entry.player",
                "account": "entry"
              }
            ]
          }
        },
        {
          "name": "destination",
          "writable": true
        }
      ],
      "args": []
    },
    {
      "name": "refundRoom",
      "discriminator": [
        244,
        238,
        123,
        48,
        50,
        50,
        210,
        133
      ],
      "accounts": [
        {
          "name": "payout",
          "accounts": [
            {
              "name": "room",
              "writable": true
            },
            {
              "name": "mint",
              "optional": true
            },
            {
              "name": "escrowTokens",
              "writable": true,
              "optional": true
            },
            {
              "name": "tokenProgram",
              "optional": true
            }
          ]
        }
      ],
      "args": []
    },
    {
      "name": "rejectResults",
      "discriminator": [
        195,
        74,
        205,
        54,
        148,
        43,
        73,
        36
      ],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "cup",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  117,
                  112
                ]
              },
              {
                "kind": "account",
                "path": "cup.id",
                "account": "cup"
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "returnGuarantee",
      "discriminator": [
        183,
        6,
        131,
        224,
        4,
        2,
        173,
        212
      ],
      "accounts": [
        {
          "name": "vault",
          "accounts": [
            {
              "name": "cup",
              "writable": true,
              "pda": {
                "seeds": [
                  {
                    "kind": "const",
                    "value": [
                      99,
                      117,
                      112
                    ]
                  },
                  {
                    "kind": "account",
                    "path": "cup.id",
                    "account": "cup"
                  }
                ]
              }
            },
            {
              "name": "mint",
              "optional": true
            },
            {
              "name": "escrowTokens",
              "writable": true,
              "optional": true
            },
            {
              "name": "tokenProgram",
              "optional": true
            }
          ]
        },
        {
          "name": "destination",
          "writable": true
        }
      ],
      "args": []
    },
    {
      "name": "revokeResolvers",
      "discriminator": [
        112,
        33,
        233,
        92,
        224,
        26,
        133,
        59
      ],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "setHolder",
      "discriminator": [
        134,
        64,
        69,
        190,
        40,
        121,
        172,
        87
      ],
      "accounts": [
        {
          "name": "admin",
          "signer": true,
          "relations": [
            "config"
          ]
        },
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "holderMint",
          "type": "pubkey"
        },
        {
          "name": "holderMin",
          "type": "u64"
        },
        {
          "name": "holderRoomFeeBps",
          "type": "u16"
        },
        {
          "name": "hostMin",
          "type": "u64"
        }
      ]
    },
    {
      "name": "setHost",
      "discriminator": [
        27,
        130,
        218,
        98,
        75,
        15,
        83,
        186
      ],
      "accounts": [
        {
          "name": "admin",
          "writable": true,
          "signer": true,
          "relations": [
            "config"
          ]
        },
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "host",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  115,
                  116
                ]
              },
              {
                "kind": "arg",
                "path": "wallet"
              }
            ]
          }
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "wallet",
          "type": "pubkey"
        },
        {
          "name": "until",
          "type": "i64"
        }
      ]
    },
    {
      "name": "setPaused",
      "discriminator": [
        91,
        60,
        125,
        192,
        176,
        225,
        166,
        218
      ],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "paused",
          "type": "bool"
        }
      ]
    },
    {
      "name": "settleRoom",
      "discriminator": [
        179,
        107,
        121,
        5,
        100,
        162,
        27,
        104
      ],
      "accounts": [
        {
          "name": "resolver",
          "signer": true
        },
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "payout",
          "accounts": [
            {
              "name": "room",
              "writable": true
            },
            {
              "name": "mint",
              "optional": true
            },
            {
              "name": "escrowTokens",
              "writable": true,
              "optional": true
            },
            {
              "name": "tokenProgram",
              "optional": true
            }
          ]
        },
        {
          "name": "treasury",
          "writable": true
        }
      ],
      "args": [
        {
          "name": "timesMs",
          "type": {
            "vec": "u32"
          }
        },
        {
          "name": "recordHash",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        }
      ]
    },
    {
      "name": "updateConfig",
      "discriminator": [
        29,
        158,
        252,
        191,
        10,
        83,
        219,
        99
      ],
      "accounts": [
        {
          "name": "admin",
          "signer": true,
          "relations": [
            "config"
          ]
        },
        {
          "name": "config",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "windows",
          "type": {
            "defined": {
              "name": "windowParams"
            }
          }
        }
      ]
    },
    {
      "name": "updateMint",
      "discriminator": [
        212,
        203,
        57,
        78,
        75,
        245,
        222,
        5
      ],
      "accounts": [
        {
          "name": "admin",
          "signer": true,
          "relations": [
            "config"
          ]
        },
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "allowedMint",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  109,
                  105,
                  110,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "allowedMint.mint",
                "account": "allowedMint"
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "mintParams"
            }
          }
        },
        {
          "name": "enabled",
          "type": "bool"
        }
      ]
    },
    {
      "name": "updateRuleset",
      "discriminator": [
        163,
        247,
        48,
        173,
        18,
        174,
        20,
        154
      ],
      "accounts": [
        {
          "name": "admin",
          "signer": true,
          "relations": [
            "config"
          ]
        },
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "ruleset",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  114,
                  117,
                  108,
                  101,
                  115,
                  101,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "ruleset.id",
                "account": "ruleset"
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "enabled",
          "type": "bool"
        }
      ]
    },
    {
      "name": "voidRoom",
      "discriminator": [
        239,
        131,
        171,
        185,
        166,
        114,
        199,
        101
      ],
      "accounts": [
        {
          "name": "resolver",
          "signer": true
        },
        {
          "name": "config",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  111,
                  110,
                  102,
                  105,
                  103
                ]
              }
            ]
          }
        },
        {
          "name": "payout",
          "accounts": [
            {
              "name": "room",
              "writable": true
            },
            {
              "name": "mint",
              "optional": true
            },
            {
              "name": "escrowTokens",
              "writable": true,
              "optional": true
            },
            {
              "name": "tokenProgram",
              "optional": true
            }
          ]
        }
      ],
      "args": [
        {
          "name": "reason",
          "type": "u8"
        },
        {
          "name": "recordHash",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "allowedMint",
      "discriminator": [
        173,
        229,
        179,
        46,
        121,
        164,
        247,
        6
      ]
    },
    {
      "name": "config",
      "discriminator": [
        155,
        12,
        170,
        224,
        30,
        250,
        204,
        130
      ]
    },
    {
      "name": "cup",
      "discriminator": [
        71,
        249,
        44,
        247,
        22,
        4,
        162,
        124
      ]
    },
    {
      "name": "entry",
      "discriminator": [
        63,
        18,
        152,
        113,
        215,
        246,
        221,
        250
      ]
    },
    {
      "name": "host",
      "discriminator": [
        141,
        195,
        28,
        93,
        45,
        127,
        175,
        216
      ]
    },
    {
      "name": "room",
      "discriminator": [
        156,
        199,
        67,
        27,
        222,
        23,
        185,
        94
      ]
    },
    {
      "name": "ruleset",
      "discriminator": [
        123,
        92,
        136,
        166,
        160,
        236,
        248,
        180
      ]
    }
  ],
  "events": [
    {
      "name": "configChanged",
      "discriminator": [
        147,
        25,
        86,
        98,
        98,
        77,
        78,
        192
      ]
    },
    {
      "name": "cupCancelled",
      "discriminator": [
        249,
        146,
        214,
        230,
        99,
        253,
        211,
        60
      ]
    },
    {
      "name": "cupClosed",
      "discriminator": [
        188,
        73,
        45,
        122,
        153,
        209,
        179,
        64
      ]
    },
    {
      "name": "cupCreated",
      "discriminator": [
        174,
        150,
        78,
        167,
        27,
        200,
        175,
        249
      ]
    },
    {
      "name": "cupEntered",
      "discriminator": [
        136,
        120,
        199,
        140,
        67,
        170,
        172,
        149
      ]
    },
    {
      "name": "cupFinalized",
      "discriminator": [
        67,
        140,
        184,
        249,
        25,
        20,
        45,
        120
      ]
    },
    {
      "name": "entryRefunded",
      "discriminator": [
        34,
        82,
        130,
        116,
        93,
        139,
        188,
        234
      ]
    },
    {
      "name": "prizeClaimed",
      "discriminator": [
        213,
        150,
        192,
        76,
        199,
        33,
        212,
        38
      ]
    },
    {
      "name": "resolversRevoked",
      "discriminator": [
        237,
        241,
        101,
        238,
        58,
        5,
        50,
        5
      ]
    },
    {
      "name": "resultsPosted",
      "discriminator": [
        149,
        119,
        76,
        166,
        231,
        230,
        205,
        144
      ]
    },
    {
      "name": "resultsRejected",
      "discriminator": [
        251,
        178,
        85,
        166,
        56,
        156,
        218,
        82
      ]
    },
    {
      "name": "rolesProposed",
      "discriminator": [
        137,
        208,
        50,
        184,
        253,
        49,
        90,
        241
      ]
    },
    {
      "name": "roomClosed",
      "discriminator": [
        150,
        23,
        85,
        195,
        169,
        114,
        30,
        88
      ]
    },
    {
      "name": "roomCreated",
      "discriminator": [
        9,
        177,
        128,
        166,
        26,
        19,
        14,
        243
      ]
    },
    {
      "name": "roomJoined",
      "discriminator": [
        72,
        221,
        8,
        87,
        75,
        143,
        178,
        49
      ]
    },
    {
      "name": "roomLeft",
      "discriminator": [
        52,
        122,
        158,
        91,
        239,
        100,
        11,
        192
      ]
    },
    {
      "name": "roomLocked",
      "discriminator": [
        139,
        9,
        142,
        55,
        220,
        219,
        98,
        158
      ]
    },
    {
      "name": "roomRefunded",
      "discriminator": [
        130,
        254,
        121,
        123,
        177,
        246,
        157,
        201
      ]
    },
    {
      "name": "roomSeeded",
      "discriminator": [
        124,
        226,
        56,
        72,
        13,
        176,
        83,
        69
      ]
    },
    {
      "name": "roomSettled",
      "discriminator": [
        139,
        64,
        121,
        102,
        137,
        174,
        165,
        105
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "unauthorized",
      "msg": "Signer is not allowed to do this"
    },
    {
      "code": 6001,
      "name": "paused",
      "msg": "New rooms, joins, cups and entries are paused"
    },
    {
      "code": 6002,
      "name": "invalidParams",
      "msg": "A parameter is out of range"
    },
    {
      "code": 6003,
      "name": "feeTooHigh",
      "msg": "Fee is above the hard cap"
    },
    {
      "code": 6004,
      "name": "invalidWindow",
      "msg": "Window is outside the allowed bounds"
    },
    {
      "code": 6005,
      "name": "rulesetDisabled",
      "msg": "Ruleset is disabled"
    },
    {
      "code": 6006,
      "name": "mintDisabled",
      "msg": "Token is not enabled for rooms and cups"
    },
    {
      "code": 6007,
      "name": "stakeOutOfRange",
      "msg": "Stake is outside this token's limits"
    },
    {
      "code": 6008,
      "name": "unsupportedMint",
      "msg": "Token mint has an extension the escrow does not support"
    },
    {
      "code": 6009,
      "name": "transferFeeTooHigh",
      "msg": "The token's transfer fee is above what this escrow accepts"
    },
    {
      "code": 6010,
      "name": "tokenAccountsMismatch",
      "msg": "Token accounts were missing, extra, or did not match the room's token"
    },
    {
      "code": 6011,
      "name": "depositMismatch",
      "msg": "Escrow received a different amount than the stake"
    },
    {
      "code": 6012,
      "name": "insufficientEscrow",
      "msg": "Escrow cannot cover this payment"
    },
    {
      "code": 6013,
      "name": "overflow",
      "msg": "Arithmetic overflow"
    },
    {
      "code": 6014,
      "name": "gateRequired",
      "msg": "The gate's signature is required"
    },
    {
      "code": 6015,
      "name": "roomNotOpen",
      "msg": "Room is not open"
    },
    {
      "code": 6016,
      "name": "roomNotLocked",
      "msg": "Room is not locked"
    },
    {
      "code": 6017,
      "name": "roomFull",
      "msg": "Room is already full"
    },
    {
      "code": 6018,
      "name": "alreadyJoined",
      "msg": "Player has already joined"
    },
    {
      "code": 6019,
      "name": "notAPlayer",
      "msg": "Signer is not a player in this room"
    },
    {
      "code": 6020,
      "name": "creatorCannotLeave",
      "msg": "The creator cannot leave; cancel the room instead"
    },
    {
      "code": 6021,
      "name": "joinClosed",
      "msg": "Join deadline has passed"
    },
    {
      "code": 6022,
      "name": "tooEarly",
      "msg": "Deadline has not passed yet"
    },
    {
      "code": 6023,
      "name": "tooLate",
      "msg": "Deadline has passed"
    },
    {
      "code": 6024,
      "name": "notEnoughPlayers",
      "msg": "A room needs at least two players to start"
    },
    {
      "code": 6025,
      "name": "invalidResults",
      "msg": "Results must give one time per player"
    },
    {
      "code": 6026,
      "name": "payoutAccountsMismatch",
      "msg": "Payout accounts do not match the players"
    },
    {
      "code": 6027,
      "name": "notRefundable",
      "msg": "Nothing to refund in this state"
    },
    {
      "code": 6028,
      "name": "notClosable",
      "msg": "Account cannot be closed yet"
    },
    {
      "code": 6029,
      "name": "badUpgrades",
      "msg": "The upgrades account is not this player's"
    },
    {
      "code": 6030,
      "name": "ratingTooHigh",
      "msg": "This car's rating is above the room's cap"
    },
    {
      "code": 6031,
      "name": "notAHolder",
      "msg": "The holder account does not qualify"
    },
    {
      "code": 6032,
      "name": "cupNotOpen",
      "msg": "Cup is not open"
    },
    {
      "code": 6033,
      "name": "cupFull",
      "msg": "Cup is full"
    },
    {
      "code": 6034,
      "name": "noEntries",
      "msg": "This cup takes no entries"
    },
    {
      "code": 6035,
      "name": "resultsNotPosted",
      "msg": "Results have not been posted"
    },
    {
      "code": 6036,
      "name": "notFinalized",
      "msg": "Cup is not finalized"
    },
    {
      "code": 6037,
      "name": "invalidRank",
      "msg": "Rank is outside the paid places"
    },
    {
      "code": 6038,
      "name": "alreadyClaimed",
      "msg": "Prize already claimed"
    },
    {
      "code": 6039,
      "name": "invalidProof",
      "msg": "Merkle proof does not match the posted results"
    },
    {
      "code": 6040,
      "name": "entryRefunded",
      "msg": "Entry was refunded"
    },
    {
      "code": 6041,
      "name": "invalidPayoutTable",
      "msg": "Payout table must have 1-32 places summing to 10000 bps"
    },
    {
      "code": 6042,
      "name": "uriTooLong",
      "msg": "Results URI is too long"
    },
    {
      "code": 6043,
      "name": "notPendingAdmin",
      "msg": "No admin change is pending for this key"
    },
    {
      "code": 6044,
      "name": "alreadySet",
      "msg": "This can only be set once"
    },
    {
      "code": 6045,
      "name": "seedDrawn",
      "msg": "The room's seed is drawn already"
    },
    {
      "code": 6046,
      "name": "seedNotDrawn",
      "msg": "The room's seed isn't drawn yet"
    },
    {
      "code": 6047,
      "name": "seedExpired",
      "msg": "The slot the room locked in is too old to draw its seed from: it can only be voided or refunded"
    },
    {
      "code": 6048,
      "name": "stakeChanged",
      "msg": "The room's stake isn't the one expected"
    },
    {
      "code": 6049,
      "name": "notLockable",
      "msg": "Only a room its creator starts can be locked; before its join deadline"
    },
    {
      "code": 6050,
      "name": "nothingPending",
      "msg": "Nothing is pending"
    },
    {
      "code": 6051,
      "name": "memoRequired",
      "msg": "A payout account requires a memo: pass the memo program as the last remaining account"
    }
  ],
  "types": [
    {
      "name": "allowedMint",
      "docs": [
        "A token allowed for rooms and cups, with its limits and fees.",
        "PDA `[\"mint\", mint]`. SOL itself is `mint = NATIVE_SOL`."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "tokenProgram",
            "docs": [
              "Owner program of the mint (Token or Token-2022); System Program for SOL."
            ],
            "type": "pubkey"
          },
          {
            "name": "decimals",
            "type": "u8"
          },
          {
            "name": "minStake",
            "type": "u64"
          },
          {
            "name": "maxStake",
            "type": "u64"
          },
          {
            "name": "roomFeeBps",
            "docs": [
              "House fee on a room's pot in this token, and on a cup's entries."
            ],
            "type": "u16"
          },
          {
            "name": "cupFeeBps",
            "type": "u16"
          },
          {
            "name": "transferFee",
            "docs": [
              "The mint carries a transfer fee (the project token): deposits are grossed",
              "up and new deposits stop if its fee goes above `max_transfer_fee_bps`."
            ],
            "type": "bool"
          },
          {
            "name": "maxTransferFeeBps",
            "type": "u16"
          },
          {
            "name": "holderMaxStake",
            "docs": [
              "The holder fee applies only to rooms staking at most this much (renting a",
              "balance to dodge fees must cost more than it saves)."
            ],
            "type": "u64"
          },
          {
            "name": "enabled",
            "type": "bool"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "config",
      "docs": [
        "Protocol settings. One per deployment, PDA `[\"config\"]`."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "admin",
            "type": "pubkey"
          },
          {
            "name": "pendingAdmin",
            "docs": [
              "Two-step admin transfer: `propose_admin` sets this, the new key accepts.",
              "`Pubkey::default()` = nothing pending."
            ],
            "type": "pubkey"
          },
          {
            "name": "guardian",
            "docs": [
              "Emergency key: may pause, reject posted cup results, cancel cups and revoke",
              "every resolver. Never moves money anywhere except back to its owners."
            ],
            "type": "pubkey"
          },
          {
            "name": "gate",
            "docs": [
              "Co-signs every new room, join and cup entry (ping, region, age, invite",
              "checks off-chain). `Pubkey::default()` = ungated. It can only admit money;",
              "it can never block an exit."
            ],
            "type": "pubkey"
          },
          {
            "name": "resolvers",
            "docs": [
              "Keys that lock, settle and void rooms and post cup results (the race",
              "server). Checked against the CURRENT set when used, so rotating a key never",
              "strands a room in flight."
            ],
            "type": {
              "vec": "pubkey"
            }
          },
          {
            "name": "treasury",
            "docs": [
              "The house treasury PDA: every fee goes here (its token accounts for SPL)."
            ],
            "type": "pubkey"
          },
          {
            "name": "prizeVault",
            "docs": [
              "The house prize-vault PDA: escrow surpluses go here, and it may organize",
              "funded cups (the house program signs for it)."
            ],
            "type": "pubkey"
          },
          {
            "name": "holderMint",
            "docs": [
              "Holder perks: the project token, set once (`Pubkey::default()` = none yet)."
            ],
            "type": "pubkey"
          },
          {
            "name": "holderMin",
            "docs": [
              "A room created by a wallet holding at least this many base units of",
              "`holder_mint` gets `holder_room_fee_bps` (if lower than its mint's fee)."
            ],
            "type": "u64"
          },
          {
            "name": "holderRoomFeeBps",
            "type": "u16"
          },
          {
            "name": "hostMin",
            "docs": [
              "A wallet holding at least this many may organize community cups."
            ],
            "type": "u64"
          },
          {
            "name": "settleWindowSecs",
            "docs": [
              "A locked room must be settled or voided within this; after it anyone",
              "refunds every stake, no fee."
            ],
            "type": "u32"
          },
          {
            "name": "challengeWindowSecs",
            "type": "u32"
          },
          {
            "name": "resultsWindowSecs",
            "type": "u32"
          },
          {
            "name": "claimWindowSecs",
            "type": "u32"
          },
          {
            "name": "paused",
            "docs": [
              "Stops new rooms, joins, cups and entries. Never stops settling, refunds,",
              "claims or closing."
            ],
            "type": "bool"
          },
          {
            "name": "cupCount",
            "docs": [
              "Cups made so far (a counter: a cup's id is its creator's choice)."
            ],
            "type": "u64"
          },
          {
            "name": "pendingGuardian",
            "docs": [
              "A new guardian, gate and resolver set, in force from `roles_after`",
              "(`ROLES_TIMELOCK_SECS` after it was proposed; 0 = nothing pending)."
            ],
            "type": "pubkey"
          },
          {
            "name": "pendingGate",
            "type": "pubkey"
          },
          {
            "name": "pendingResolvers",
            "type": {
              "vec": "pubkey"
            }
          },
          {
            "name": "rolesAfter",
            "type": "i64"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "configChanged",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "admin",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "configParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "guardian",
            "type": "pubkey"
          },
          {
            "name": "gate",
            "type": "pubkey"
          },
          {
            "name": "treasury",
            "type": "pubkey"
          },
          {
            "name": "prizeVault",
            "type": "pubkey"
          },
          {
            "name": "settleWindowSecs",
            "type": "u32"
          },
          {
            "name": "challengeWindowSecs",
            "type": "u32"
          },
          {
            "name": "resultsWindowSecs",
            "type": "u32"
          },
          {
            "name": "claimWindowSecs",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "cup",
      "docs": [
        "A prize event: a leaderboard over a window, an optional entry fee, a payout",
        "table, an optional guarantee and an optional featured stock.",
        "PDA `[\"cup\", id_le]`; escrow is the PDA (SOL) or its ATA."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "id",
            "type": "u64"
          },
          {
            "name": "kind",
            "type": {
              "defined": {
                "name": "cupKind"
              }
            }
          },
          {
            "name": "class",
            "type": "u8"
          },
          {
            "name": "organizer",
            "type": "pubkey"
          },
          {
            "name": "sponsor",
            "docs": [
              "Paid the guarantee; gets back what isn't won."
            ],
            "type": "pubkey"
          },
          {
            "name": "rentPayer",
            "docs": [
              "Paid the cup's rent (the organizer, or the house keeper); gets it back."
            ],
            "type": "pubkey"
          },
          {
            "name": "host",
            "docs": [
              "A community cup's host (`Pubkey::default()` = none) and its share of the",
              "house fee, paid at close."
            ],
            "type": "pubkey"
          },
          {
            "name": "hostShareBps",
            "type": "u16"
          },
          {
            "name": "ruleset",
            "type": "pubkey"
          },
          {
            "name": "specHash",
            "docs": [
              "sha256 of the event's race spec, published with it."
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "featuredMint",
            "docs": [
              "The event's featured stock: winners may swap their prize into it in their",
              "own claim transaction. Informational: the program never holds it."
            ],
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "tokenProgram",
            "type": "pubkey"
          },
          {
            "name": "transferFee",
            "type": "bool"
          },
          {
            "name": "entryFee",
            "type": "u64"
          },
          {
            "name": "feeBps",
            "type": "u16"
          },
          {
            "name": "treasury",
            "type": "pubkey"
          },
          {
            "name": "prizeVault",
            "docs": [
              "Anything left above what the cup owed goes here at close."
            ],
            "type": "pubkey"
          },
          {
            "name": "maxTransferFeeBps",
            "docs": [
              "New entries stop if the token's transfer fee goes above this."
            ],
            "type": "u16"
          },
          {
            "name": "payoutBps",
            "docs": [
              "Share of the pool per place, in bps, summing to 10000."
            ],
            "type": {
              "vec": "u16"
            }
          },
          {
            "name": "guarantee",
            "docs": [
              "Minimum pool, deposited by `sponsor` at creation."
            ],
            "type": "u64"
          },
          {
            "name": "startsAt",
            "type": "i64"
          },
          {
            "name": "endsAt",
            "type": "i64"
          },
          {
            "name": "resultsDeadline",
            "docs": [
              "Results must be posted before this, or every entry can be refunded."
            ],
            "type": "i64"
          },
          {
            "name": "challengeWindowSecs",
            "type": "u32"
          },
          {
            "name": "claimWindowSecs",
            "type": "u32"
          },
          {
            "name": "maxEntrants",
            "docs": [
              "0 = unlimited."
            ],
            "type": "u32"
          },
          {
            "name": "entrants",
            "type": "u32"
          },
          {
            "name": "refunded",
            "type": "u32"
          },
          {
            "name": "state",
            "type": {
              "defined": {
                "name": "cupState"
              }
            }
          },
          {
            "name": "resultsRoot",
            "docs": [
              "Merkle root over leaves hash(0x00 ‖ cup ‖ rank_le16 ‖ wallet) for every",
              "paid place. Ranks only: the program computes every amount."
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "scorers",
            "type": "u32"
          },
          {
            "name": "standingsHash",
            "docs": [
              "Hash of the full published standings (every ranked run and its record)."
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "resultsUri",
            "type": "string"
          },
          {
            "name": "postedAt",
            "type": "i64"
          },
          {
            "name": "challengeEnds",
            "type": "i64"
          },
          {
            "name": "claimUntil",
            "docs": [
              "Before this only the winner may claim (and swap into the featured stock);",
              "after it anyone may push the prize to the winner."
            ],
            "type": "i64"
          },
          {
            "name": "pool",
            "docs": [
              "Set by `finalize`."
            ],
            "type": "u64"
          },
          {
            "name": "places",
            "type": "u8"
          },
          {
            "name": "usedBps",
            "type": "u32"
          },
          {
            "name": "claimed",
            "docs": [
              "Bit r-1 set = rank r paid."
            ],
            "type": "u32"
          },
          {
            "name": "paid",
            "type": "u64"
          },
          {
            "name": "guaranteeReturned",
            "type": "bool"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "cupCancelled",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "cup",
            "type": "pubkey"
          },
          {
            "name": "by",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "cupClosed",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "cup",
            "type": "pubkey"
          },
          {
            "name": "toSponsor",
            "type": "u64"
          },
          {
            "name": "toHost",
            "type": "u64"
          },
          {
            "name": "toTreasury",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "cupCreated",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "cup",
            "type": "pubkey"
          },
          {
            "name": "id",
            "type": "u64"
          },
          {
            "name": "organizer",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "entryFee",
            "type": "u64"
          },
          {
            "name": "guarantee",
            "type": "u64"
          },
          {
            "name": "featuredMint",
            "type": "pubkey"
          },
          {
            "name": "endsAt",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "cupEntered",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "cup",
            "type": "pubkey"
          },
          {
            "name": "player",
            "type": "pubkey"
          },
          {
            "name": "entrants",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "cupFinalized",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "cup",
            "type": "pubkey"
          },
          {
            "name": "pool",
            "type": "u64"
          },
          {
            "name": "places",
            "type": "u8"
          },
          {
            "name": "claimUntil",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "cupKind",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "entries"
          },
          {
            "name": "board"
          }
        ]
      }
    },
    {
      "name": "cupParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "kind",
            "type": {
              "defined": {
                "name": "cupKind"
              }
            }
          },
          {
            "name": "class",
            "type": "u8"
          },
          {
            "name": "entryFee",
            "type": "u64"
          },
          {
            "name": "payoutBps",
            "type": {
              "vec": "u16"
            }
          },
          {
            "name": "guarantee",
            "type": "u64"
          },
          {
            "name": "startsAt",
            "type": "i64"
          },
          {
            "name": "endsAt",
            "type": "i64"
          },
          {
            "name": "maxEntrants",
            "type": "u32"
          },
          {
            "name": "specHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "featuredMint",
            "docs": [
              "The event's featured stock (`Pubkey::default()` = none)."
            ],
            "type": "pubkey"
          },
          {
            "name": "hostShareBps",
            "docs": [
              "Community cups only: the host's share of the house fee."
            ],
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "cupState",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "open"
          },
          {
            "name": "posted"
          },
          {
            "name": "finalized"
          },
          {
            "name": "cancelled"
          }
        ]
      }
    },
    {
      "name": "entry",
      "docs": [
        "One player's cup entry. PDA `[\"entry\", cup, player]`."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "cup",
            "type": "pubkey"
          },
          {
            "name": "player",
            "type": "pubkey"
          },
          {
            "name": "rentPayer",
            "docs": [
              "Paid the entry's rent (the race server's sponsor, or the player); gets it back."
            ],
            "type": "pubkey"
          },
          {
            "name": "refunded",
            "type": "bool"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "entryRefunded",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "cup",
            "type": "pubkey"
          },
          {
            "name": "player",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "host",
      "docs": [
        "A wallet the admin lets organize community cups (the path with no token).",
        "PDA `[\"host\", wallet]`."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "wallet",
            "type": "pubkey"
          },
          {
            "name": "until",
            "docs": [
              "Hosting allowed until this unix time."
            ],
            "type": "i64"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "mintParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "minStake",
            "type": "u64"
          },
          {
            "name": "maxStake",
            "type": "u64"
          },
          {
            "name": "roomFeeBps",
            "type": "u16"
          },
          {
            "name": "cupFeeBps",
            "type": "u16"
          },
          {
            "name": "allowTransferFee",
            "docs": [
              "Accept a transfer-fee mint (the project token), up to this fee per transfer."
            ],
            "type": "bool"
          },
          {
            "name": "maxTransferFeeBps",
            "type": "u16"
          },
          {
            "name": "holderMaxStake",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "prizeClaimed",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "cup",
            "type": "pubkey"
          },
          {
            "name": "rank",
            "type": "u16"
          },
          {
            "name": "wallet",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "pushed",
            "docs": [
              "The winner signed (and may have swapped it into the featured stock), or",
              "it was pushed after the claim window."
            ],
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "raceSpec",
      "docs": [
        "What race a room runs: the race server builds exactly this race."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "track",
            "type": "u8"
          },
          {
            "name": "format",
            "docs": [
              "0 sprint, 1 grand prix."
            ],
            "type": "u8"
          },
          {
            "name": "laps",
            "type": "u8"
          },
          {
            "name": "ai",
            "docs": [
              "AI cars on the grid."
            ],
            "type": "u8"
          },
          {
            "name": "time",
            "docs": [
              "0 day, 1 sunset, 2 night."
            ],
            "type": "u8"
          },
          {
            "name": "sky",
            "docs": [
              "0 clear, 1 cloudy, 2 light, 3 rain, 4 storm, 5 changing."
            ],
            "type": "u8"
          },
          {
            "name": "startMode",
            "docs": [
              "0 starts when full, 1 the creator starts it (two players or more)."
            ],
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "resolversRevoked",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "by",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "resultsPosted",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "cup",
            "type": "pubkey"
          },
          {
            "name": "resultsRoot",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "scorers",
            "type": "u32"
          },
          {
            "name": "standingsHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "resultsUri",
            "type": "string"
          },
          {
            "name": "challengeEnds",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "resultsRejected",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "cup",
            "type": "pubkey"
          },
          {
            "name": "by",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "rolesProposed",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "guardian",
            "type": "pubkey"
          },
          {
            "name": "gate",
            "type": "pubkey"
          },
          {
            "name": "after",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "room",
      "docs": [
        "A live race's pot: everyone stakes the same, the race server runs the race,",
        "the resolver settles it and the program pays the fastest at once.",
        "PDA `[\"room\", code]`, where `code` is the race server's 8-character room id.",
        "The escrow is the PDA itself (SOL) or its associated token account.",
        "",
        "The first fields form a fixed prefix after the 8-byte discriminator, so the",
        "lobby can filter with memcmp: state @8, access @9, class @10, mint @11,",
        "stake @43, max_players @51, max_rating @52."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "state",
            "type": {
              "defined": {
                "name": "roomState"
              }
            }
          },
          {
            "name": "access",
            "type": "u8"
          },
          {
            "name": "class",
            "type": "u8"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "stake",
            "type": "u64"
          },
          {
            "name": "maxPlayers",
            "type": "u8"
          },
          {
            "name": "maxRating",
            "docs": [
              "GARAGE rooms: the highest car rating allowed, in tenths; 0 = no cap."
            ],
            "type": "u16"
          },
          {
            "name": "code",
            "type": {
              "array": [
                "u8",
                8
              ]
            }
          },
          {
            "name": "creator",
            "type": "pubkey"
          },
          {
            "name": "ruleset",
            "type": "pubkey"
          },
          {
            "name": "race",
            "type": {
              "defined": {
                "name": "raceSpec"
              }
            }
          },
          {
            "name": "tokenProgram",
            "type": "pubkey"
          },
          {
            "name": "feeBps",
            "docs": [
              "Terms frozen at creation: config changes never touch a room in flight."
            ],
            "type": "u16"
          },
          {
            "name": "transferFee",
            "type": "bool"
          },
          {
            "name": "maxTransferFeeBps",
            "docs": [
              "New stakes stop if the token's transfer fee goes above this."
            ],
            "type": "u16"
          },
          {
            "name": "treasury",
            "type": "pubkey"
          },
          {
            "name": "prizeVault",
            "type": "pubkey"
          },
          {
            "name": "rentPayer",
            "docs": [
              "Paid the room's rent (the server's sponsor, or the creator); gets it back."
            ],
            "type": "pubkey"
          },
          {
            "name": "players",
            "docs": [
              "Seat order = the race's grid order."
            ],
            "type": {
              "vec": "pubkey"
            }
          },
          {
            "name": "levels",
            "docs": [
              "GARAGE rooms: each player's upgrade levels when they joined."
            ],
            "type": {
              "vec": {
                "array": [
                  "u8",
                  6
                ]
              }
            }
          },
          {
            "name": "seed",
            "docs": [
              "Drawn after the room locks (`draw_seed`): hash(room ‖ the hash of the slot",
              "it locked in ‖ that slot). All zeros until then."
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "timesMs",
            "docs": [
              "Final times in seat order, set at settlement. `DNF` = no result."
            ],
            "type": {
              "vec": "u32"
            }
          },
          {
            "name": "winners",
            "docs": [
              "Bit i set = players[i] was paid a winner's share."
            ],
            "type": "u8"
          },
          {
            "name": "share",
            "type": "u64"
          },
          {
            "name": "toTreasury",
            "type": "u64"
          },
          {
            "name": "recordHash",
            "docs": [
              "sha256 of the race's full record (canonical JSON), published by the race",
              "server: anyone can run the race again from it and check these times."
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "voidReason",
            "type": "u8"
          },
          {
            "name": "settledBy",
            "type": "pubkey"
          },
          {
            "name": "createdAt",
            "type": "i64"
          },
          {
            "name": "joinDeadline",
            "type": "i64"
          },
          {
            "name": "settleWindowSecs",
            "type": "u32"
          },
          {
            "name": "lockedAt",
            "type": "i64"
          },
          {
            "name": "lockedSlot",
            "docs": [
              "The slot the room locked in: its hash, unknown while the locking",
              "transaction ran, is the seed's source."
            ],
            "type": "u64"
          },
          {
            "name": "settleDeadline",
            "docs": [
              "After this (locked), anyone can refund every stake, no fee."
            ],
            "type": "i64"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "roomClosed",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "room",
            "type": "pubkey"
          },
          {
            "name": "surplus",
            "docs": [
              "Anything above what the room owed (a donation, a reward pushed to the",
              "escrow), sent to the prize vault."
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "roomCreated",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "room",
            "type": "pubkey"
          },
          {
            "name": "code",
            "type": {
              "array": [
                "u8",
                8
              ]
            }
          },
          {
            "name": "creator",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "stake",
            "type": "u64"
          },
          {
            "name": "maxPlayers",
            "type": "u8"
          },
          {
            "name": "feeBps",
            "type": "u16"
          },
          {
            "name": "class",
            "type": "u8"
          },
          {
            "name": "joinDeadline",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "roomJoined",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "room",
            "type": "pubkey"
          },
          {
            "name": "player",
            "type": "pubkey"
          },
          {
            "name": "players",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "roomLeft",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "room",
            "type": "pubkey"
          },
          {
            "name": "player",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "roomLocked",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "room",
            "type": "pubkey"
          },
          {
            "name": "lockedSlot",
            "docs": [
              "The seed comes from this slot's hash, once it is over (`draw_seed`)."
            ],
            "type": "u64"
          },
          {
            "name": "players",
            "type": "u8"
          },
          {
            "name": "settleDeadline",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "roomParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "stake",
            "type": "u64"
          },
          {
            "name": "maxPlayers",
            "type": "u8"
          },
          {
            "name": "joinWindowSecs",
            "type": "u32"
          },
          {
            "name": "class",
            "type": "u8"
          },
          {
            "name": "maxRating",
            "docs": [
              "GARAGE only: the highest rating allowed, in tenths (0 = no cap)."
            ],
            "type": "u16"
          },
          {
            "name": "access",
            "type": "u8"
          },
          {
            "name": "race",
            "type": {
              "defined": {
                "name": "raceSpec"
              }
            }
          }
        ]
      }
    },
    {
      "name": "roomRefunded",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "room",
            "type": "pubkey"
          },
          {
            "name": "reason",
            "type": "u8"
          },
          {
            "name": "recordHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "by",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "roomSeeded",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "room",
            "type": "pubkey"
          },
          {
            "name": "seed",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    },
    {
      "name": "roomSettled",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "room",
            "type": "pubkey"
          },
          {
            "name": "timesMs",
            "type": {
              "vec": "u32"
            }
          },
          {
            "name": "recordHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "winners",
            "type": "u8"
          },
          {
            "name": "share",
            "docs": [
              "What each winner was sent from escrow (before any transfer fee)."
            ],
            "type": "u64"
          },
          {
            "name": "toTreasury",
            "type": "u64"
          },
          {
            "name": "settledBy",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "roomState",
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "open"
          },
          {
            "name": "locked"
          },
          {
            "name": "settled"
          },
          {
            "name": "refunded"
          }
        ]
      }
    },
    {
      "name": "ruleset",
      "docs": [
        "One version of the game's rules (physics, track, race rules, upgrade",
        "effects), hashed by the game build. Rooms and cups name the ruleset they are",
        "raced under, so a new sim version can never re-rule a race in flight.",
        "PDA `[\"ruleset\", id_le]`."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "id",
            "type": "u32"
          },
          {
            "name": "hash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "ratingPoints",
            "docs": [
              "A GARAGE car's rating points per department level (`src/game/upgrades.js`",
              "order: pu, aero, tyre, brake, chassis, dur), in tenths of a rating point."
            ],
            "type": {
              "array": [
                "u8",
                6
              ]
            }
          },
          {
            "name": "enabled",
            "type": "bool"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "windowParams",
      "docs": [
        "The windows the admin may change at once (they only reach FUTURE rooms and",
        "cups, which freeze their own)."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "settleWindowSecs",
            "type": "u32"
          },
          {
            "name": "challengeWindowSecs",
            "type": "u32"
          },
          {
            "name": "resultsWindowSecs",
            "type": "u32"
          },
          {
            "name": "claimWindowSecs",
            "type": "u32"
          }
        ]
      }
    }
  ]
};
