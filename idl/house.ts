/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/house.json`.
 */
export type House = {
  "address": "5rV9yXr14uvsxJZnNqQRRwX7ZZ3FWNqZcGyrz3B4YXgw",
  "metadata": {
    "name": "house",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "ACME house: the treasury split, the prize vault and the upgrade shop"
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
          "name": "house",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  117,
                  115,
                  101
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
            "house"
          ]
        },
        {
          "name": "house",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  117,
                  115,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "priceTable",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  105,
                  99,
                  101
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
          "name": "totals",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  111,
                  116,
                  97,
                  108,
                  115
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
        }
      ]
    },
    {
      "name": "applyFundCaps",
      "discriminator": [
        119,
        186,
        43,
        228,
        79,
        5,
        205,
        244
      ],
      "accounts": [
        {
          "name": "house",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  117,
                  115,
                  101
                ]
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "applySplit",
      "discriminator": [
        52,
        174,
        230,
        206,
        227,
        242,
        206,
        174
      ],
      "accounts": [
        {
          "name": "house",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  117,
                  115,
                  101
                ]
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "applyTokenMint",
      "discriminator": [
        174,
        153,
        19,
        222,
        210,
        23,
        255,
        143
      ],
      "accounts": [
        {
          "name": "house",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  117,
                  115,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "pending",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  111,
                  107,
                  101,
                  110
                ]
              }
            ]
          }
        },
        {
          "name": "mint"
        }
      ],
      "args": []
    },
    {
      "name": "buyLot",
      "discriminator": [
        13,
        244,
        229,
        160,
        48,
        32,
        34,
        63
      ],
      "accounts": [
        {
          "name": "buyer",
          "docs": [
            "Burns the price from their own account; receives the lot (SOL: here)."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "house",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  117,
                  115,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "auction",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  97,
                  117,
                  99,
                  116,
                  105,
                  111,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "auction.lotMint",
                "account": "auction"
              }
            ]
          }
        },
        {
          "name": "tokenTotals",
          "docs": [
            "The project token's running totals (what was burned)."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  111,
                  116,
                  97,
                  108,
                  115
                ]
              },
              {
                "kind": "account",
                "path": "house.tokenMint",
                "account": "houseConfig"
              }
            ]
          }
        },
        {
          "name": "tokenMint",
          "docs": [
            "The project token's mint: the burn takes its supply down."
          ],
          "writable": true
        },
        {
          "name": "buyerTokens",
          "docs": [
            "The buyer's account of the project token, burned from."
          ],
          "writable": true
        },
        {
          "name": "tokenProgram"
        },
        {
          "name": "buyback",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  117,
                  121,
                  98,
                  97,
                  99,
                  107
                ]
              }
            ]
          }
        },
        {
          "name": "lotMint",
          "docs": [
            "Token lots: the lot's mint, the vault's account of it, the buyer's",
            "(owned by the buyer) and their token program."
          ],
          "optional": true
        },
        {
          "name": "buybackTokens",
          "writable": true,
          "optional": true
        },
        {
          "name": "buyerLotTokens",
          "writable": true,
          "optional": true
        },
        {
          "name": "lotTokenProgram",
          "optional": true
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "maxPrice",
          "type": "u64"
        },
        {
          "name": "minLot",
          "type": "u64"
        }
      ]
    },
    {
      "name": "buyUpgrade",
      "discriminator": [
        136,
        43,
        33,
        208,
        15,
        32,
        65,
        61
      ],
      "accounts": [
        {
          "name": "owner",
          "writable": true,
          "signer": true
        },
        {
          "name": "payer",
          "docs": [
            "Pays the upgrades account's rent on a first purchase (the race server's",
            "sponsor, or the owner)."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "house",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  117,
                  115,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "priceTable",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  105,
                  99,
                  101
                ]
              },
              {
                "kind": "account",
                "path": "priceTable.mint",
                "account": "priceTable"
              }
            ]
          }
        },
        {
          "name": "upgrades",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  117,
                  112,
                  103,
                  114,
                  97,
                  100,
                  101,
                  115
                ]
              },
              {
                "kind": "account",
                "path": "owner"
              }
            ]
          }
        },
        {
          "name": "treasury",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  101,
                  97,
                  115,
                  117,
                  114,
                  121
                ]
              }
            ]
          }
        },
        {
          "name": "mint",
          "optional": true
        },
        {
          "name": "ownerTokens",
          "writable": true,
          "optional": true
        },
        {
          "name": "treasuryTokens",
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
          "name": "dept",
          "type": "u8"
        },
        {
          "name": "level",
          "type": "u8"
        },
        {
          "name": "maxPrice",
          "type": "u64"
        }
      ]
    },
    {
      "name": "fundCup",
      "discriminator": [
        109,
        48,
        32,
        166,
        249,
        72,
        62,
        103
      ],
      "accounts": [
        {
          "name": "keeper",
          "docs": [
            "Pays the cup's rent."
          ],
          "writable": true,
          "signer": true
        },
        {
          "name": "house",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  117,
                  115,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "prize",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  105,
                  122,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "totals",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  111,
                  116,
                  97,
                  108,
                  115
                ]
              },
              {
                "kind": "account",
                "path": "totals.mint",
                "account": "totals"
              }
            ]
          }
        },
        {
          "name": "arenaConfig",
          "writable": true
        },
        {
          "name": "ruleset"
        },
        {
          "name": "allowedMint"
        },
        {
          "name": "cup",
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
          "name": "prizeTokens",
          "writable": true,
          "optional": true
        },
        {
          "name": "tokenProgram",
          "optional": true
        },
        {
          "name": "arenaProgram",
          "address": "G8hRPxkadXhZcuyeH7vWCqmfAqfWNAuxJnBftPezcUdK"
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
          "name": "house",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  117,
                  115,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "treasury",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  101,
                  97,
                  115,
                  117,
                  114,
                  121
                ]
              }
            ]
          }
        },
        {
          "name": "prize",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  105,
                  122,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "buyback",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  117,
                  121,
                  98,
                  97,
                  99,
                  107
                ]
              }
            ]
          }
        },
        {
          "name": "program",
          "address": "5rV9yXr14uvsxJZnNqQRRwX7ZZ3FWNqZcGyrz3B4YXgw"
        },
        {
          "name": "programData",
          "docs": [
            "Only the upgrade authority may bootstrap the house."
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
              "name": "houseParams"
            }
          }
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
            "house"
          ]
        },
        {
          "name": "house",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  117,
                  115,
                  101
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
      "name": "proposeSplit",
      "discriminator": [
        38,
        104,
        30,
        96,
        107,
        245,
        76,
        252
      ],
      "accounts": [
        {
          "name": "admin",
          "signer": true,
          "relations": [
            "house"
          ]
        },
        {
          "name": "house",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  117,
                  115,
                  101
                ]
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "split",
          "type": {
            "defined": {
              "name": "split"
            }
          }
        }
      ]
    },
    {
      "name": "proposeTokenMint",
      "discriminator": [
        108,
        142,
        4,
        218,
        236,
        118,
        124,
        253
      ],
      "accounts": [
        {
          "name": "admin",
          "writable": true,
          "signer": true,
          "relations": [
            "house"
          ]
        },
        {
          "name": "house",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  117,
                  115,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "pending",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  111,
                  107,
                  101,
                  110
                ]
              }
            ]
          }
        },
        {
          "name": "mint",
          "docs": [
            "The project token's mint itself."
          ]
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "setAuctionFloor",
      "discriminator": [
        222,
        7,
        9,
        59,
        159,
        23,
        206,
        16
      ],
      "accounts": [
        {
          "name": "admin",
          "writable": true,
          "signer": true,
          "relations": [
            "house"
          ]
        },
        {
          "name": "house",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  117,
                  115,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "auction",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  97,
                  117,
                  99,
                  116,
                  105,
                  111,
                  110
                ]
              },
              {
                "kind": "arg",
                "path": "lotMint"
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
          "name": "lotMint",
          "type": "pubkey"
        },
        {
          "name": "minLot",
          "type": "u64"
        },
        {
          "name": "minPrice",
          "type": "u64"
        }
      ]
    },
    {
      "name": "setFundCaps",
      "discriminator": [
        21,
        210,
        213,
        71,
        22,
        114,
        133,
        201
      ],
      "accounts": [
        {
          "name": "admin",
          "signer": true,
          "relations": [
            "house"
          ]
        },
        {
          "name": "house",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  117,
                  115,
                  101
                ]
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "fundMaxBps",
          "type": "u16"
        },
        {
          "name": "fundEverySecs",
          "type": "u32"
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
          "name": "admin",
          "signer": true,
          "relations": [
            "house"
          ]
        },
        {
          "name": "house",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  117,
                  115,
                  101
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
      "name": "setPrices",
      "discriminator": [
        155,
        1,
        215,
        185,
        100,
        8,
        177,
        186
      ],
      "accounts": [
        {
          "name": "authority",
          "docs": [
            "The admin alone: levels are never refunded, so a price only the admin can",
            "set is a price a leaked server key can't drop to nothing."
          ],
          "signer": true
        },
        {
          "name": "house",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  117,
                  115,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "priceTable",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  105,
                  99,
                  101
                ]
              },
              {
                "kind": "account",
                "path": "priceTable.mint",
                "account": "priceTable"
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "prices",
          "type": {
            "array": [
              {
                "array": [
                  "u64",
                  5
                ]
              },
              6
            ]
          }
        },
        {
          "name": "enabled",
          "type": "bool"
        }
      ]
    },
    {
      "name": "setRoles",
      "discriminator": [
        119,
        86,
        129,
        161,
        55,
        23,
        250,
        12
      ],
      "accounts": [
        {
          "name": "admin",
          "signer": true,
          "relations": [
            "house"
          ]
        },
        {
          "name": "house",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  117,
                  115,
                  101
                ]
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "keeper",
          "type": "pubkey"
        },
        {
          "name": "opsWallet",
          "type": "pubkey"
        }
      ]
    },
    {
      "name": "startAuction",
      "discriminator": [
        255,
        2,
        149,
        136,
        148,
        125,
        65,
        195
      ],
      "accounts": [
        {
          "name": "keeper",
          "signer": true
        },
        {
          "name": "house",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  117,
                  115,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "auction",
          "docs": [
            "Made with the admin's lowest rate (`set_auction_floor`): none, no auction."
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  97,
                  117,
                  99,
                  116,
                  105,
                  111,
                  110
                ]
              },
              {
                "kind": "arg",
                "path": "lotMint"
              }
            ]
          }
        },
        {
          "name": "buyback",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  117,
                  121,
                  98,
                  97,
                  99,
                  107
                ]
              }
            ]
          }
        },
        {
          "name": "buybackTokens",
          "docs": [
            "Token lots: the buyback vault's account of `lot_mint`."
          ],
          "optional": true
        }
      ],
      "args": [
        {
          "name": "lotMint",
          "type": "pubkey"
        },
        {
          "name": "lot",
          "type": "u64"
        },
        {
          "name": "startPrice",
          "type": "u64"
        },
        {
          "name": "floorPrice",
          "type": "u64"
        },
        {
          "name": "declineSecs",
          "type": "u32"
        },
        {
          "name": "runSecs",
          "type": "u32"
        }
      ]
    },
    {
      "name": "sweep",
      "discriminator": [
        40,
        23,
        234,
        175,
        14,
        61,
        154,
        177
      ],
      "accounts": [
        {
          "name": "house",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  104,
                  111,
                  117,
                  115,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "treasury",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  114,
                  101,
                  97,
                  115,
                  117,
                  114,
                  121
                ]
              }
            ]
          }
        },
        {
          "name": "prize",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  105,
                  122,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "buyback",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  98,
                  117,
                  121,
                  98,
                  97,
                  99,
                  107
                ]
              }
            ]
          }
        },
        {
          "name": "opsWallet",
          "writable": true
        },
        {
          "name": "totals",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  111,
                  116,
                  97,
                  108,
                  115
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
            "Token sweeps: the mint (writable: the project token's share is burned) and",
            "the four parties' token accounts of it (the buyback's only when it gets a",
            "share that isn't burned)."
          ],
          "writable": true,
          "optional": true
        },
        {
          "name": "treasuryTokens",
          "writable": true,
          "optional": true
        },
        {
          "name": "opsTokens",
          "writable": true,
          "optional": true
        },
        {
          "name": "prizeTokens",
          "writable": true,
          "optional": true
        },
        {
          "name": "buybackTokens",
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
          "name": "mintKey",
          "type": "pubkey"
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "auction",
      "discriminator": [
        218,
        94,
        247,
        242,
        126,
        233,
        131,
        81
      ]
    },
    {
      "name": "houseConfig",
      "discriminator": [
        194,
        237,
        216,
        79,
        135,
        254,
        7,
        212
      ]
    },
    {
      "name": "pendingToken",
      "discriminator": [
        235,
        175,
        151,
        228,
        131,
        159,
        37,
        181
      ]
    },
    {
      "name": "priceTable",
      "discriminator": [
        249,
        145,
        86,
        106,
        7,
        90,
        110,
        77
      ]
    },
    {
      "name": "totals",
      "discriminator": [
        200,
        56,
        239,
        35,
        57,
        206,
        72,
        74
      ]
    },
    {
      "name": "upgrades",
      "discriminator": [
        124,
        166,
        118,
        161,
        178,
        46,
        191,
        24
      ]
    }
  ],
  "events": [
    {
      "name": "auctionFloorSet",
      "discriminator": [
        189,
        232,
        43,
        222,
        27,
        171,
        75,
        53
      ]
    },
    {
      "name": "auctionStarted",
      "discriminator": [
        126,
        97,
        193,
        56,
        72,
        162,
        162,
        64
      ]
    },
    {
      "name": "cupFunded",
      "discriminator": [
        213,
        228,
        143,
        250,
        181,
        27,
        73,
        78
      ]
    },
    {
      "name": "houseChanged",
      "discriminator": [
        96,
        221,
        188,
        133,
        132,
        26,
        203,
        108
      ]
    },
    {
      "name": "lotBought",
      "discriminator": [
        203,
        144,
        224,
        166,
        111,
        44,
        231,
        154
      ]
    },
    {
      "name": "pricesSet",
      "discriminator": [
        245,
        27,
        72,
        221,
        199,
        127,
        134,
        160
      ]
    },
    {
      "name": "splitApplied",
      "discriminator": [
        225,
        133,
        252,
        170,
        157,
        41,
        49,
        221
      ]
    },
    {
      "name": "splitProposed",
      "discriminator": [
        79,
        18,
        65,
        165,
        147,
        244,
        179,
        235
      ]
    },
    {
      "name": "swept",
      "discriminator": [
        254,
        138,
        9,
        198,
        192,
        61,
        165,
        135
      ]
    },
    {
      "name": "tokenMintApplied",
      "discriminator": [
        118,
        73,
        115,
        15,
        37,
        205,
        31,
        215
      ]
    },
    {
      "name": "tokenMintProposed",
      "discriminator": [
        207,
        31,
        212,
        170,
        161,
        126,
        77,
        29
      ]
    },
    {
      "name": "upgradeBought",
      "discriminator": [
        142,
        82,
        129,
        123,
        102,
        228,
        164,
        240
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
      "msg": "Sales are paused"
    },
    {
      "code": 6002,
      "name": "invalidParams",
      "msg": "A parameter is out of range"
    },
    {
      "code": 6003,
      "name": "invalidSplit",
      "msg": "The split must add up to 10000 bps"
    },
    {
      "code": 6004,
      "name": "tooEarly",
      "msg": "The timelock has not passed yet"
    },
    {
      "code": 6005,
      "name": "nothingPending",
      "msg": "No split change is pending"
    },
    {
      "code": 6006,
      "name": "notPendingAdmin",
      "msg": "No admin change is pending for this key"
    },
    {
      "code": 6007,
      "name": "alreadySet",
      "msg": "This can only be set once"
    },
    {
      "code": 6008,
      "name": "tokenAccountsMismatch",
      "msg": "Token accounts were missing, extra, or did not match"
    },
    {
      "code": 6009,
      "name": "notForSale",
      "msg": "Not for sale in this token"
    },
    {
      "code": 6010,
      "name": "wrongLevel",
      "msg": "Levels are bought one at a time, in order"
    },
    {
      "code": 6011,
      "name": "priceTooHigh",
      "msg": "The price is above the buyer's limit"
    },
    {
      "code": 6012,
      "name": "fundTooLarge",
      "msg": "The guarantee is above what the prize vault may fund"
    },
    {
      "code": 6013,
      "name": "fundTooSoon",
      "msg": "Cups were funded too recently"
    },
    {
      "code": 6014,
      "name": "overflow",
      "msg": "Arithmetic overflow"
    },
    {
      "code": 6015,
      "name": "notABoardCup",
      "msg": "The prize vault funds only free board cups"
    },
    {
      "code": 6016,
      "name": "transferFeeTooHigh",
      "msg": "The token's transfer fee is above what the house accepts"
    },
    {
      "code": 6017,
      "name": "noToken",
      "msg": "There is no project token yet"
    },
    {
      "code": 6018,
      "name": "notTheToken",
      "msg": "That isn't the project token"
    },
    {
      "code": 6019,
      "name": "auctionRunning",
      "msg": "An auction of this token is still running"
    },
    {
      "code": 6020,
      "name": "auctionTooSoon",
      "msg": "The last auction of this token started less than a day ago"
    },
    {
      "code": 6021,
      "name": "noAuction",
      "msg": "Nothing is for sale in this auction"
    },
    {
      "code": 6022,
      "name": "lotTooLarge",
      "msg": "The lot is above a quarter of what the buyback vault holds"
    },
    {
      "code": 6023,
      "name": "floorTooLow",
      "msg": "The floor is below half the last sale's price"
    },
    {
      "code": 6024,
      "name": "lotTooSmall",
      "msg": "The lot is smaller than the buyer's minimum"
    },
    {
      "code": 6025,
      "name": "vaultShort",
      "msg": "The buyback vault can't cover the lot"
    }
  ],
  "types": [
    {
      "name": "auction",
      "docs": [
        "The buyback auction of one token (or SOL): the buyback vault's balance sold,",
        "a lot at a time, for the project token, which the buyer burns from their own",
        "wallet. The price, in the project token, for the whole lot, falls evenly",
        "from `start_price` at `start_at` to `floor_price` over `decline_secs` and",
        "holds there until `end_at`. No oracle: the falling price finds the market.",
        "PDA `[\"auction\", lot_mint]` (`NATIVE_SOL` for SOL)."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "lotMint",
            "type": "pubkey"
          },
          {
            "name": "lot",
            "docs": [
              "For sale now (0: nothing; an auction past `end_at` is over too)."
            ],
            "type": "u64"
          },
          {
            "name": "startPrice",
            "type": "u64"
          },
          {
            "name": "floorPrice",
            "type": "u64"
          },
          {
            "name": "startAt",
            "type": "i64"
          },
          {
            "name": "declineSecs",
            "type": "u32"
          },
          {
            "name": "endAt",
            "type": "i64"
          },
          {
            "name": "minLot",
            "docs": [
              "The admin's lowest rate: no floor prices the lot below `min_price` of",
              "the project token per `min_lot` of it (`set_auction_floor`)."
            ],
            "type": "u64"
          },
          {
            "name": "minPrice",
            "type": "u64"
          },
          {
            "name": "lastLot",
            "docs": [
              "The last sale's lot, price and time: for `ANCHOR_SECS` after it, a new",
              "floor is held to at least `FLOOR_ANCHOR_BPS` of its rate."
            ],
            "type": "u64"
          },
          {
            "name": "lastPrice",
            "type": "u64"
          },
          {
            "name": "lastSoldAt",
            "type": "i64"
          },
          {
            "name": "sold",
            "docs": [
              "Running totals: lots sold, what they paid out, the project token burned."
            ],
            "type": "u32"
          },
          {
            "name": "paidOut",
            "type": "u64"
          },
          {
            "name": "burned",
            "type": "u64"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "auctionFloorSet",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "lotMint",
            "type": "pubkey"
          },
          {
            "name": "minLot",
            "type": "u64"
          },
          {
            "name": "minPrice",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "auctionStarted",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "lotMint",
            "type": "pubkey"
          },
          {
            "name": "lot",
            "type": "u64"
          },
          {
            "name": "startPrice",
            "type": "u64"
          },
          {
            "name": "floorPrice",
            "type": "u64"
          },
          {
            "name": "startAt",
            "type": "i64"
          },
          {
            "name": "declineSecs",
            "type": "u32"
          },
          {
            "name": "endAt",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "cupFunded",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "cup",
            "type": "pubkey"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "guarantee",
            "type": "u64"
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
      "name": "houseChanged",
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
      "name": "houseConfig",
      "docs": [
        "The house's settings. PDA `[\"house\"]`."
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
            "type": "pubkey"
          },
          {
            "name": "keeper",
            "docs": [
              "May fund weekly cups (within the caps)."
            ],
            "type": "pubkey"
          },
          {
            "name": "opsWallet",
            "docs": [
              "Where the studio's share goes (its token accounts for SPL)."
            ],
            "type": "pubkey"
          },
          {
            "name": "split",
            "type": {
              "defined": {
                "name": "split"
              }
            }
          },
          {
            "name": "pendingSplit",
            "docs": [
              "A new split takes effect only after `split_after` (a public timelock)."
            ],
            "type": {
              "defined": {
                "name": "split"
              }
            }
          },
          {
            "name": "splitAfter",
            "type": "i64"
          },
          {
            "name": "tokenMint",
            "docs": [
              "The project token, set ONCE (`Pubkey::default()` = none yet): its share of",
              "fees is burned, and the buyback share of other tokens builds up for it."
            ],
            "type": "pubkey"
          },
          {
            "name": "fundMaxBps",
            "docs": [
              "A funded cup's guarantee may take at most this share of the prize vault's",
              "balance in that token, and at most once per `fund_every_secs`."
            ],
            "type": "u16"
          },
          {
            "name": "fundEverySecs",
            "type": "u32"
          },
          {
            "name": "pendingFundMaxBps",
            "docs": [
              "Looser caps, in force from `fund_caps_after` (the split's timelock; 0 =",
              "nothing pending). Tighter ones apply at once."
            ],
            "type": "u16"
          },
          {
            "name": "pendingFundEverySecs",
            "type": "u32"
          },
          {
            "name": "fundCapsAfter",
            "type": "i64"
          },
          {
            "name": "lastFundAt",
            "docs": [
              "When a cup was last funded (`i64::MIN`: never)."
            ],
            "type": "i64"
          },
          {
            "name": "paused",
            "docs": [
              "Stops sales and cup funding. Never stops sweeping or anything that pays out."
            ],
            "type": "bool"
          },
          {
            "name": "treasuryBump",
            "type": "u8"
          },
          {
            "name": "prizeBump",
            "type": "u8"
          },
          {
            "name": "buybackBump",
            "type": "u8"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "houseParams",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "keeper",
            "type": "pubkey"
          },
          {
            "name": "opsWallet",
            "type": "pubkey"
          },
          {
            "name": "split",
            "type": {
              "defined": {
                "name": "split"
              }
            }
          },
          {
            "name": "fundMaxBps",
            "type": "u16"
          },
          {
            "name": "fundEverySecs",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "lotBought",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "lotMint",
            "type": "pubkey"
          },
          {
            "name": "lot",
            "type": "u64"
          },
          {
            "name": "price",
            "docs": [
              "The project token the buyer burned."
            ],
            "type": "u64"
          },
          {
            "name": "buyer",
            "type": "pubkey"
          },
          {
            "name": "slot",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "pendingToken",
      "docs": [
        "The project token, proposed and waiting out `TOKEN_TIMELOCK_SECS` before",
        "anyone applies it to the house. PDA `[\"token\"]`."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "after",
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
      "name": "priceTable",
      "docs": [
        "Upgrade prices in one token: department × level 1-5, in base units",
        "(0 = not for sale). PDA `[\"price\", mint]`; SOL is `mint = NATIVE_SOL`."
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
            "type": "pubkey"
          },
          {
            "name": "decimals",
            "type": "u8"
          },
          {
            "name": "prices",
            "type": {
              "array": [
                {
                  "array": [
                    "u64",
                    5
                  ]
                },
                6
              ]
            }
          },
          {
            "name": "enabled",
            "type": "bool"
          },
          {
            "name": "updatedAt",
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
      "name": "pricesSet",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "enabled",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "split",
      "docs": [
        "How every fee and sale is split by `sweep`, in bps of 10000."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "opsBps",
            "docs": [
              "The studio's running costs: servers, the audit, development."
            ],
            "type": "u16"
          },
          {
            "name": "prizeBps",
            "docs": [
              "The prize vault: the weekly stock prizes and other funded cups."
            ],
            "type": "u16"
          },
          {
            "name": "buybackBps",
            "docs": [
              "The project token's buyback (burned); until the token exists, prizes."
            ],
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "splitApplied",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "split",
            "type": {
              "defined": {
                "name": "split"
              }
            }
          }
        ]
      }
    },
    {
      "name": "splitProposed",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "split",
            "type": {
              "defined": {
                "name": "split"
              }
            }
          },
          {
            "name": "after",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "swept",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "toOps",
            "type": "u64"
          },
          {
            "name": "toPrize",
            "type": "u64"
          },
          {
            "name": "toBuyback",
            "type": "u64"
          },
          {
            "name": "burned",
            "docs": [
              "The project token's buyback share, burned instead of kept."
            ],
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "tokenMintApplied",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "tokenMintProposed",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
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
      "name": "totals",
      "docs": [
        "Running totals per token, for anyone to read (the project page shows them).",
        "PDA `[\"totals\", mint]`."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "sweptOps",
            "type": "u64"
          },
          {
            "name": "sweptPrize",
            "type": "u64"
          },
          {
            "name": "sweptBuyback",
            "type": "u64"
          },
          {
            "name": "burned",
            "type": "u64"
          },
          {
            "name": "funded",
            "type": "u64"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "upgradeBought",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "dept",
            "type": "u8"
          },
          {
            "name": "level",
            "type": "u8"
          },
          {
            "name": "mint",
            "type": "pubkey"
          },
          {
            "name": "price",
            "type": "u64"
          },
          {
            "name": "levels",
            "type": {
              "array": [
                "u8",
                8
              ]
            }
          },
          {
            "name": "slot",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "upgrades",
      "docs": [
        "A player's upgrade levels: one account per wallet, for every car look.",
        "PDA `[\"upgrades\", owner]`. A fixed layout the arena program reads raw (the",
        "levels snapshot of a GARAGE room): discriminator (8) | version (1) |",
        "owner (32) | levels [u8; 8] (6 used, `src/game/upgrades.js` order: pu, aero,",
        "tyre, brake, chassis, dur) | updated_at | bump. Never a token: levels can't",
        "be sold or moved."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "version",
            "type": "u8"
          },
          {
            "name": "owner",
            "type": "pubkey"
          },
          {
            "name": "levels",
            "type": {
              "array": [
                "u8",
                8
              ]
            }
          },
          {
            "name": "updatedAt",
            "type": "i64"
          },
          {
            "name": "bump",
            "type": "u8"
          }
        ]
      }
    }
  ]
};
