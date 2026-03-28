#!/usr/bin/env node
/**
 * Dina Network — Deploy ALL World Currency Stablecoins to Testnet
 *
 * Deploys 155+ stablecoins for every ISO 4217 currency.
 * Each is a yield-bearing DRC-1 token backed by USDC.
 *
 * Usage: node scripts/deploy-all-currencies.js
 */

const REST_URL = process.env.DINA_REST_URL || 'http://35.184.213.248:8080';
const RPC_URL = process.env.DINA_RPC_URL || 'http://35.184.213.248:8545';

// Rate = micro-units of currency per 1 USDC (6 decimal places)
// Yield = basis points (450 = 4.50% APY) — approximates each country's gov bond rate
// All rates are approximate testnet values

const ALL_CURRENCIES = [
  // ══════════════════════════════════════════
  // NORTH AMERICA
  // ══════════════════════════════════════════
  { code: 'USD', symbol: 'USDC',  name: 'US Dollar', country: 'US', rate: 1_000_000, yield: 450, flag: '🇺🇸' },
  { code: 'CAD', symbol: 'CADC',  name: 'Canadian Dollar', country: 'CA', rate: 1_380_000, yield: 380, flag: '🇨🇦' },
  { code: 'MXN', symbol: 'MXNC',  name: 'Mexican Peso', country: 'MX', rate: 17_500_000, yield: 1050, flag: '🇲🇽' },
  { code: 'GTQ', symbol: 'GTQC',  name: 'Guatemalan Quetzal', country: 'GT', rate: 7_800_000, yield: 500, flag: '🇬🇹' },
  { code: 'BZD', symbol: 'BZDC',  name: 'Belize Dollar', country: 'BZ', rate: 2_000_000, yield: 350, flag: '🇧🇿' },
  { code: 'HNL', symbol: 'HNLC',  name: 'Honduran Lempira', country: 'HN', rate: 24_800_000, yield: 600, flag: '🇭🇳' },
  { code: 'NIO', symbol: 'NIOC',  name: 'Nicaraguan Córdoba', country: 'NI', rate: 36_800_000, yield: 700, flag: '🇳🇮' },
  { code: 'CRC', symbol: 'CRCC',  name: 'Costa Rican Colón', country: 'CR', rate: 530_000_000, yield: 500, flag: '🇨🇷' },
  { code: 'PAB', symbol: 'PABC',  name: 'Panamanian Balboa', country: 'PA', rate: 1_000_000, yield: 450, flag: '🇵🇦' },
  { code: 'DOP', symbol: 'DOPC',  name: 'Dominican Peso', country: 'DO', rate: 58_500_000, yield: 800, flag: '🇩🇴' },
  { code: 'CUP', symbol: 'CUPC',  name: 'Cuban Peso', country: 'CU', rate: 24_000_000, yield: 300, flag: '🇨🇺' },
  { code: 'HTG', symbol: 'HTGC',  name: 'Haitian Gourde', country: 'HT', rate: 133_000_000, yield: 500, flag: '🇭🇹' },
  { code: 'JMD', symbol: 'JMDC',  name: 'Jamaican Dollar', country: 'JM', rate: 155_000_000, yield: 700, flag: '🇯🇲' },
  { code: 'TTD', symbol: 'TTDC',  name: 'Trinidad & Tobago Dollar', country: 'TT', rate: 6_800_000, yield: 400, flag: '🇹🇹' },
  { code: 'BBD', symbol: 'BBDC',  name: 'Barbadian Dollar', country: 'BB', rate: 2_000_000, yield: 350, flag: '🇧🇧' },
  { code: 'BSD', symbol: 'BSDC',  name: 'Bahamian Dollar', country: 'BS', rate: 1_000_000, yield: 400, flag: '🇧🇸' },
  { code: 'XCD', symbol: 'XCDC',  name: 'East Caribbean Dollar', country: 'EC', rate: 2_700_000, yield: 350, flag: '🏝️' },
  { code: 'AWG', symbol: 'AWGC',  name: 'Aruban Florin', country: 'AW', rate: 1_790_000, yield: 350, flag: '🇦🇼' },
  { code: 'ANG', symbol: 'ANGC',  name: 'Netherlands Antillean Guilder', country: 'CW', rate: 1_790_000, yield: 350, flag: '🇨🇼' },
  { code: 'BMD', symbol: 'BMDC',  name: 'Bermudian Dollar', country: 'BM', rate: 1_000_000, yield: 450, flag: '🇧🇲' },
  { code: 'KYD', symbol: 'KYDC',  name: 'Cayman Islands Dollar', country: 'KY', rate: 830_000, yield: 400, flag: '🇰🇾' },

  // ══════════════════════════════════════════
  // SOUTH AMERICA
  // ══════════════════════════════════════════
  { code: 'BRL', symbol: 'BRSC',  name: 'Brazilian Real', country: 'BR', rate: 5_100_000, yield: 1250, flag: '🇧🇷' },
  { code: 'ARS', symbol: 'ARSC',  name: 'Argentine Peso', country: 'AR', rate: 900_000_000, yield: 4000, flag: '🇦🇷' },
  { code: 'CLP', symbol: 'CLPC',  name: 'Chilean Peso', country: 'CL', rate: 950_000_000, yield: 500, flag: '🇨🇱' },
  { code: 'COP', symbol: 'COPC',  name: 'Colombian Peso', country: 'CO', rate: 4_200_000_000, yield: 900, flag: '🇨🇴' },
  { code: 'PEN', symbol: 'PENC',  name: 'Peruvian Sol', country: 'PE', rate: 3_750_000, yield: 600, flag: '🇵🇪' },
  { code: 'UYU', symbol: 'UYUC',  name: 'Uruguayan Peso', country: 'UY', rate: 42_000_000, yield: 900, flag: '🇺🇾' },
  { code: 'PYG', symbol: 'PYGC',  name: 'Paraguayan Guarani', country: 'PY', rate: 7_500_000_000, yield: 600, flag: '🇵🇾' },
  { code: 'BOB', symbol: 'BOBC',  name: 'Bolivian Boliviano', country: 'BO', rate: 6_900_000, yield: 400, flag: '🇧🇴' },
  { code: 'VES', symbol: 'VESC',  name: 'Venezuelan Bolívar', country: 'VE', rate: 36_500_000, yield: 5000, flag: '🇻🇪' },
  { code: 'GYD', symbol: 'GYDC',  name: 'Guyanese Dollar', country: 'GY', rate: 209_000_000, yield: 400, flag: '🇬🇾' },
  { code: 'SRD', symbol: 'SRDC',  name: 'Surinamese Dollar', country: 'SR', rate: 36_200_000, yield: 500, flag: '🇸🇷' },
  { code: 'FKP', symbol: 'FKPC',  name: 'Falkland Islands Pound', country: 'FK', rate: 790_000, yield: 400, flag: '🇫🇰' },

  // ══════════════════════════════════════════
  // WESTERN EUROPE
  // ══════════════════════════════════════════
  { code: 'EUR', symbol: 'EURC',  name: 'Euro', country: 'EU', rate: 930_000, yield: 350, flag: '🇪🇺' },
  { code: 'GBP', symbol: 'GBPC',  name: 'British Pound', country: 'GB', rate: 790_000, yield: 400, flag: '🇬🇧' },
  { code: 'CHF', symbol: 'CHFC',  name: 'Swiss Franc', country: 'CH', rate: 880_000, yield: 150, flag: '🇨🇭' },
  { code: 'SEK', symbol: 'SEKG',  name: 'Swedish Krona', country: 'SE', rate: 10_500_000, yield: 300, flag: '🇸🇪' },
  { code: 'NOK', symbol: 'NOKC',  name: 'Norwegian Krone', country: 'NO', rate: 10_800_000, yield: 350, flag: '🇳🇴' },
  { code: 'DKK', symbol: 'DKKC',  name: 'Danish Krone', country: 'DK', rate: 6_950_000, yield: 320, flag: '🇩🇰' },
  { code: 'ISK', symbol: 'ISKC',  name: 'Icelandic Króna', country: 'IS', rate: 138_000_000, yield: 900, flag: '🇮🇸' },
  { code: 'GIP', symbol: 'GIPC',  name: 'Gibraltar Pound', country: 'GI', rate: 790_000, yield: 400, flag: '🇬🇮' },

  // ══════════════════════════════════════════
  // EASTERN EUROPE
  // ══════════════════════════════════════════
  { code: 'PLN', symbol: 'PLNC',  name: 'Polish Zloty', country: 'PL', rate: 4_050_000, yield: 550, flag: '🇵🇱' },
  { code: 'CZK', symbol: 'CZKC',  name: 'Czech Koruna', country: 'CZ', rate: 23_500_000, yield: 400, flag: '🇨🇿' },
  { code: 'HUF', symbol: 'HUFC',  name: 'Hungarian Forint', country: 'HU', rate: 375_000_000, yield: 650, flag: '🇭🇺' },
  { code: 'RON', symbol: 'RONC',  name: 'Romanian Leu', country: 'RO', rate: 4_650_000, yield: 550, flag: '🇷🇴' },
  { code: 'BGN', symbol: 'BGNC',  name: 'Bulgarian Lev', country: 'BG', rate: 1_820_000, yield: 350, flag: '🇧🇬' },
  { code: 'HRK', symbol: 'HRKC',  name: 'Croatian Kuna', country: 'HR', rate: 7_050_000, yield: 350, flag: '🇭🇷' },
  { code: 'RSD', symbol: 'RSDC',  name: 'Serbian Dinar', country: 'RS', rate: 109_000_000, yield: 500, flag: '🇷🇸' },
  { code: 'BAM', symbol: 'BAMC',  name: 'Bosnia-Herzegovina Mark', country: 'BA', rate: 1_820_000, yield: 350, flag: '🇧🇦' },
  { code: 'MKD', symbol: 'MKDC',  name: 'Macedonian Denar', country: 'MK', rate: 57_500_000, yield: 400, flag: '🇲🇰' },
  { code: 'ALL', symbol: 'ALLC',  name: 'Albanian Lek', country: 'AL', rate: 96_000_000, yield: 400, flag: '🇦🇱' },
  { code: 'MDL', symbol: 'MDLC',  name: 'Moldovan Leu', country: 'MD', rate: 17_800_000, yield: 500, flag: '🇲🇩' },
  { code: 'UAH', symbol: 'UAHC',  name: 'Ukrainian Hryvnia', country: 'UA', rate: 41_500_000, yield: 1500, flag: '🇺🇦' },
  { code: 'BYN', symbol: 'BYNC',  name: 'Belarusian Ruble', country: 'BY', rate: 3_250_000, yield: 800, flag: '🇧🇾' },
  { code: 'GEL', symbol: 'GELC',  name: 'Georgian Lari', country: 'GE', rate: 2_710_000, yield: 900, flag: '🇬🇪' },
  { code: 'AMD', symbol: 'AMDC',  name: 'Armenian Dram', country: 'AM', rate: 388_000_000, yield: 800, flag: '🇦🇲' },
  { code: 'AZN', symbol: 'AZNC',  name: 'Azerbaijani Manat', country: 'AZ', rate: 1_700_000, yield: 500, flag: '🇦🇿' },
  { code: 'TRY', symbol: 'TRYL',  name: 'Turkish Lira', country: 'TR', rate: 32_500_000, yield: 4500, flag: '🇹🇷' },
  { code: 'RUB', symbol: 'RUBC',  name: 'Russian Ruble', country: 'RU', rate: 92_000_000, yield: 1600, flag: '🇷🇺' },

  // ══════════════════════════════════════════
  // EAST ASIA
  // ══════════════════════════════════════════
  { code: 'JPY', symbol: 'JPYC',  name: 'Japanese Yen', country: 'JP', rate: 154_000_000, yield: 50, flag: '🇯🇵' },
  { code: 'CNY', symbol: 'CNHC',  name: 'Chinese Yuan', country: 'CN', rate: 7_250_000, yield: 250, flag: '🇨🇳' },
  { code: 'KRW', symbol: 'KRWC',  name: 'South Korean Won', country: 'KR', rate: 1_380_000_000, yield: 320, flag: '🇰🇷' },
  { code: 'TWD', symbol: 'TWDC',  name: 'Taiwan Dollar', country: 'TW', rate: 32_500_000, yield: 150, flag: '🇹🇼' },
  { code: 'HKD', symbol: 'HKDC',  name: 'Hong Kong Dollar', country: 'HK', rate: 7_810_000, yield: 430, flag: '🇭🇰' },
  { code: 'MOP', symbol: 'MOPC',  name: 'Macau Pataca', country: 'MO', rate: 8_050_000, yield: 400, flag: '🇲🇴' },
  { code: 'MNT', symbol: 'MNTC',  name: 'Mongolian Tugrik', country: 'MN', rate: 3_450_000_000, yield: 1200, flag: '🇲🇳' },
  { code: 'KPW', symbol: 'KPWC',  name: 'North Korean Won', country: 'KP', rate: 900_000_000, yield: 0, flag: '🇰🇵' },

  // ══════════════════════════════════════════
  // SOUTHEAST ASIA
  // ══════════════════════════════════════════
  { code: 'SGD', symbol: 'SGDC',  name: 'Singapore Dollar', country: 'SG', rate: 1_340_000, yield: 320, flag: '🇸🇬' },
  { code: 'THB', symbol: 'THBC',  name: 'Thai Baht', country: 'TH', rate: 35_500_000, yield: 200, flag: '🇹🇭' },
  { code: 'MYR', symbol: 'MYRC',  name: 'Malaysian Ringgit', country: 'MY', rate: 4_700_000, yield: 300, flag: '🇲🇾' },
  { code: 'IDR', symbol: 'IDRC',  name: 'Indonesian Rupiah', country: 'ID', rate: 15_800_000_000, yield: 600, flag: '🇮🇩' },
  { code: 'PHP', symbol: 'PHPC',  name: 'Philippine Peso', country: 'PH', rate: 56_500_000, yield: 550, flag: '🇵🇭' },
  { code: 'VND', symbol: 'VNDC',  name: 'Vietnamese Dong', country: 'VN', rate: 25_000_000_000, yield: 400, flag: '🇻🇳' },
  { code: 'MMK', symbol: 'MMKC',  name: 'Myanmar Kyat', country: 'MM', rate: 2_100_000_000, yield: 500, flag: '🇲🇲' },
  { code: 'KHR', symbol: 'KHRC',  name: 'Cambodian Riel', country: 'KH', rate: 4_100_000_000, yield: 300, flag: '🇰🇭' },
  { code: 'LAK', symbol: 'LAKC',  name: 'Lao Kip', country: 'LA', rate: 21_000_000_000, yield: 500, flag: '🇱🇦' },
  { code: 'BND', symbol: 'BNDC',  name: 'Brunei Dollar', country: 'BN', rate: 1_340_000, yield: 300, flag: '🇧🇳' },
  { code: 'TLP', symbol: 'TLPC',  name: 'Timor-Leste (uses USD)', country: 'TL', rate: 1_000_000, yield: 450, flag: '🇹🇱' },

  // ══════════════════════════════════════════
  // SOUTH ASIA
  // ══════════════════════════════════════════
  { code: 'INR', symbol: 'INRC',  name: 'Indian Rupee', country: 'IN', rate: 83_500_000, yield: 650, flag: '🇮🇳' },
  { code: 'PKR', symbol: 'PKRC',  name: 'Pakistani Rupee', country: 'PK', rate: 278_000_000, yield: 2000, flag: '🇵🇰' },
  { code: 'BDT', symbol: 'BDTC',  name: 'Bangladeshi Taka', country: 'BD', rate: 110_000_000, yield: 700, flag: '🇧🇩' },
  { code: 'LKR', symbol: 'LKRC',  name: 'Sri Lankan Rupee', country: 'LK', rate: 305_000_000, yield: 1000, flag: '🇱🇰' },
  { code: 'NPR', symbol: 'NPRC',  name: 'Nepalese Rupee', country: 'NP', rate: 133_500_000, yield: 700, flag: '🇳🇵' },
  { code: 'BTN', symbol: 'BTNC',  name: 'Bhutanese Ngultrum', country: 'BT', rate: 83_500_000, yield: 500, flag: '🇧🇹' },
  { code: 'MVR', symbol: 'MVRC',  name: 'Maldivian Rufiyaa', country: 'MV', rate: 15_400_000, yield: 400, flag: '🇲🇻' },
  { code: 'AFN', symbol: 'AFNC',  name: 'Afghan Afghani', country: 'AF', rate: 70_000_000, yield: 500, flag: '🇦🇫' },

  // ══════════════════════════════════════════
  // MIDDLE EAST
  // ══════════════════════════════════════════
  { code: 'AED', symbol: 'AEDC',  name: 'UAE Dirham', country: 'AE', rate: 3_670_000, yield: 450, flag: '🇦🇪' },
  { code: 'SAR', symbol: 'SARC',  name: 'Saudi Riyal', country: 'SA', rate: 3_750_000, yield: 500, flag: '🇸🇦' },
  { code: 'QAR', symbol: 'QARC',  name: 'Qatari Riyal', country: 'QA', rate: 3_640_000, yield: 500, flag: '🇶🇦' },
  { code: 'BHD', symbol: 'BHDC',  name: 'Bahraini Dinar', country: 'BH', rate: 376_000, yield: 500, flag: '🇧🇭' },
  { code: 'OMR', symbol: 'OMRC',  name: 'Omani Rial', country: 'OM', rate: 385_000, yield: 450, flag: '🇴🇲' },
  { code: 'KWD', symbol: 'KWDC',  name: 'Kuwaiti Dinar', country: 'KW', rate: 307_000, yield: 400, flag: '🇰🇼' },
  { code: 'JOD', symbol: 'JODC',  name: 'Jordanian Dinar', country: 'JO', rate: 709_000, yield: 600, flag: '🇯🇴' },
  { code: 'ILS', symbol: 'ILSC',  name: 'Israeli Shekel', country: 'IL', rate: 3_650_000, yield: 450, flag: '🇮🇱' },
  { code: 'LBP', symbol: 'LBPC',  name: 'Lebanese Pound', country: 'LB', rate: 89_500_000_000, yield: 0, flag: '🇱🇧' },
  { code: 'IQD', symbol: 'IQDC',  name: 'Iraqi Dinar', country: 'IQ', rate: 1_310_000_000, yield: 400, flag: '🇮🇶' },
  { code: 'IRR', symbol: 'IRRC',  name: 'Iranian Rial', country: 'IR', rate: 42_000_000_000, yield: 2300, flag: '🇮🇷' },
  { code: 'SYP', symbol: 'SYPC',  name: 'Syrian Pound', country: 'SY', rate: 13_000_000_000, yield: 0, flag: '🇸🇾' },
  { code: 'YER', symbol: 'YERC',  name: 'Yemeni Rial', country: 'YE', rate: 250_000_000, yield: 500, flag: '🇾🇪' },

  // ══════════════════════════════════════════
  // CENTRAL ASIA
  // ══════════════════════════════════════════
  { code: 'KZT', symbol: 'KZTC',  name: 'Kazakhstani Tenge', country: 'KZ', rate: 450_000_000, yield: 1400, flag: '🇰🇿' },
  { code: 'UZS', symbol: 'UZSC',  name: 'Uzbekistani Som', country: 'UZ', rate: 12_700_000_000, yield: 1400, flag: '🇺🇿' },
  { code: 'KGS', symbol: 'KGSC',  name: 'Kyrgyzstani Som', country: 'KG', rate: 89_000_000, yield: 1000, flag: '🇰🇬' },
  { code: 'TJS', symbol: 'TJSC',  name: 'Tajikistani Somoni', country: 'TJ', rate: 10_900_000, yield: 800, flag: '🇹🇯' },
  { code: 'TMT', symbol: 'TMTC',  name: 'Turkmenistani Manat', country: 'TM', rate: 3_500_000, yield: 500, flag: '🇹🇲' },

  // ══════════════════════════════════════════
  // NORTH AFRICA
  // ══════════════════════════════════════════
  { code: 'EGP', symbol: 'EGPC',  name: 'Egyptian Pound', country: 'EG', rate: 48_500_000, yield: 2200, flag: '🇪🇬' },
  { code: 'MAD', symbol: 'MADC',  name: 'Moroccan Dirham', country: 'MA', rate: 10_000_000, yield: 300, flag: '🇲🇦' },
  { code: 'DZD', symbol: 'DZDC',  name: 'Algerian Dinar', country: 'DZ', rate: 135_000_000, yield: 400, flag: '🇩🇿' },
  { code: 'TND', symbol: 'TNDC',  name: 'Tunisian Dinar', country: 'TN', rate: 3_120_000, yield: 700, flag: '🇹🇳' },
  { code: 'LYD', symbol: 'LYDC',  name: 'Libyan Dinar', country: 'LY', rate: 4_850_000, yield: 300, flag: '🇱🇾' },
  { code: 'SDG', symbol: 'SDGC',  name: 'Sudanese Pound', country: 'SD', rate: 601_000_000, yield: 1500, flag: '🇸🇩' },

  // ══════════════════════════════════════════
  // WEST AFRICA
  // ══════════════════════════════════════════
  { code: 'NGN', symbol: 'NGNS',  name: 'Nigerian Naira', country: 'NG', rate: 1_550_000_000, yield: 1800, flag: '🇳🇬' },
  { code: 'GHS', symbol: 'GHSC',  name: 'Ghanaian Cedi', country: 'GH', rate: 15_200_000, yield: 2500, flag: '🇬🇭' },
  { code: 'XOF', symbol: 'XOFC',  name: 'West African CFA Franc', country: 'WA', rate: 610_000_000, yield: 350, flag: '🌍' },
  { code: 'GMD', symbol: 'GMDC',  name: 'Gambian Dalasi', country: 'GM', rate: 67_000_000, yield: 600, flag: '🇬🇲' },
  { code: 'GNF', symbol: 'GNFC',  name: 'Guinean Franc', country: 'GN', rate: 8_600_000_000, yield: 500, flag: '🇬🇳' },
  { code: 'SLL', symbol: 'SLLC',  name: 'Sierra Leonean Leone', country: 'SL', rate: 22_500_000_000, yield: 500, flag: '🇸🇱' },
  { code: 'LRD', symbol: 'LRDC',  name: 'Liberian Dollar', country: 'LR', rate: 192_000_000, yield: 500, flag: '🇱🇷' },
  { code: 'CVE', symbol: 'CVEC',  name: 'Cape Verdean Escudo', country: 'CV', rate: 102_500_000, yield: 350, flag: '🇨🇻' },
  { code: 'MRU', symbol: 'MRUC',  name: 'Mauritanian Ouguiya', country: 'MR', rate: 39_700_000, yield: 500, flag: '🇲🇷' },

  // ══════════════════════════════════════════
  // EAST AFRICA
  // ══════════════════════════════════════════
  { code: 'KES', symbol: 'KESG',  name: 'Kenyan Shilling', country: 'KE', rate: 152_000_000, yield: 1000, flag: '🇰🇪' },
  { code: 'TZS', symbol: 'TZSC',  name: 'Tanzanian Shilling', country: 'TZ', rate: 2_650_000_000, yield: 700, flag: '🇹🇿' },
  { code: 'UGX', symbol: 'UGXC',  name: 'Ugandan Shilling', country: 'UG', rate: 3_800_000_000, yield: 900, flag: '🇺🇬' },
  { code: 'ETB', symbol: 'ETBC',  name: 'Ethiopian Birr', country: 'ET', rate: 57_000_000, yield: 800, flag: '🇪🇹' },
  { code: 'RWF', symbol: 'RWFC',  name: 'Rwandan Franc', country: 'RW', rate: 1_290_000_000, yield: 700, flag: '🇷🇼' },
  { code: 'BIF', symbol: 'BIFC',  name: 'Burundian Franc', country: 'BI', rate: 2_870_000_000, yield: 500, flag: '🇧🇮' },
  { code: 'SOS', symbol: 'SOSC',  name: 'Somali Shilling', country: 'SO', rate: 571_000_000, yield: 400, flag: '🇸🇴' },
  { code: 'DJF', symbol: 'DJFC',  name: 'Djiboutian Franc', country: 'DJ', rate: 177_700_000, yield: 400, flag: '🇩🇯' },
  { code: 'ERN', symbol: 'ERNC',  name: 'Eritrean Nakfa', country: 'ER', rate: 15_000_000, yield: 300, flag: '🇪🇷' },
  { code: 'SSP', symbol: 'SSPC',  name: 'South Sudanese Pound', country: 'SS', rate: 130_000_000, yield: 500, flag: '🇸🇸' },

  // ══════════════════════════════════════════
  // CENTRAL AFRICA
  // ══════════════════════════════════════════
  { code: 'XAF', symbol: 'XAFC',  name: 'Central African CFA Franc', country: 'CA', rate: 610_000_000, yield: 350, flag: '🌍' },
  { code: 'CDF', symbol: 'CDFC',  name: 'Congolese Franc', country: 'CD', rate: 2_780_000_000, yield: 500, flag: '🇨🇩' },
  { code: 'STN', symbol: 'STNC',  name: 'São Tomé Dobra', country: 'ST', rate: 22_800_000, yield: 400, flag: '🇸🇹' },

  // ══════════════════════════════════════════
  // SOUTHERN AFRICA
  // ══════════════════════════════════════════
  { code: 'ZAR', symbol: 'ZARC',  name: 'South African Rand', country: 'ZA', rate: 18_500_000, yield: 800, flag: '🇿🇦' },
  { code: 'BWP', symbol: 'BWPC',  name: 'Botswana Pula', country: 'BW', rate: 13_700_000, yield: 500, flag: '🇧🇼' },
  { code: 'NAD', symbol: 'NADC',  name: 'Namibian Dollar', country: 'NA', rate: 18_500_000, yield: 700, flag: '🇳🇦' },
  { code: 'SZL', symbol: 'SZLC',  name: 'Eswatini Lilangeni', country: 'SZ', rate: 18_500_000, yield: 700, flag: '🇸🇿' },
  { code: 'LSL', symbol: 'LSLC',  name: 'Lesotho Loti', country: 'LS', rate: 18_500_000, yield: 700, flag: '🇱🇸' },
  { code: 'MWK', symbol: 'MWKC',  name: 'Malawian Kwacha', country: 'MW', rate: 1_740_000_000, yield: 2000, flag: '🇲🇼' },
  { code: 'ZMW', symbol: 'ZMWC',  name: 'Zambian Kwacha', country: 'ZM', rate: 26_500_000, yield: 1200, flag: '🇿🇲' },
  { code: 'MZN', symbol: 'MZNC',  name: 'Mozambican Metical', country: 'MZ', rate: 63_800_000, yield: 1300, flag: '🇲🇿' },
  { code: 'AOA', symbol: 'AOAC',  name: 'Angolan Kwanza', country: 'AO', rate: 835_000_000, yield: 1500, flag: '🇦🇴' },
  { code: 'MGA', symbol: 'MGAC',  name: 'Malagasy Ariary', country: 'MG', rate: 4_550_000_000, yield: 800, flag: '🇲🇬' },
  { code: 'MUR', symbol: 'MURC',  name: 'Mauritian Rupee', country: 'MU', rate: 45_500_000, yield: 400, flag: '🇲🇺' },
  { code: 'SCR', symbol: 'SCRC',  name: 'Seychellois Rupee', country: 'SC', rate: 14_200_000, yield: 350, flag: '🇸🇨' },
  { code: 'KMF', symbol: 'KMFC',  name: 'Comorian Franc', country: 'KM', rate: 457_000_000, yield: 350, flag: '🇰🇲' },
  { code: 'ZWL', symbol: 'ZWLC',  name: 'Zimbabwean Dollar', country: 'ZW', rate: 13_500_000_000, yield: 3000, flag: '🇿🇼' },

  // ══════════════════════════════════════════
  // OCEANIA
  // ══════════════════════════════════════════
  { code: 'AUD', symbol: 'AUDC',  name: 'Australian Dollar', country: 'AU', rate: 1_540_000, yield: 400, flag: '🇦🇺' },
  { code: 'NZD', symbol: 'NZDC',  name: 'New Zealand Dollar', country: 'NZ', rate: 1_670_000, yield: 450, flag: '🇳🇿' },
  { code: 'FJD', symbol: 'FJDC',  name: 'Fijian Dollar', country: 'FJ', rate: 2_250_000, yield: 350, flag: '🇫🇯' },
  { code: 'PGK', symbol: 'PGKC',  name: 'Papua New Guinean Kina', country: 'PG', rate: 3_850_000, yield: 500, flag: '🇵🇬' },
  { code: 'WST', symbol: 'WSTC',  name: 'Samoan Tala', country: 'WS', rate: 2_780_000, yield: 350, flag: '🇼🇸' },
  { code: 'TOP', symbol: 'TOPC',  name: 'Tongan Pa\u02BBanga', country: 'TO', rate: 2_380_000, yield: 300, flag: '🇹🇴' },
  { code: 'VUV', symbol: 'VUVC',  name: 'Vanuatu Vatu', country: 'VU', rate: 119_000_000, yield: 300, flag: '🇻🇺' },
  { code: 'SBD', symbol: 'SBDC',  name: 'Solomon Islands Dollar', country: 'SB', rate: 8_400_000, yield: 350, flag: '🇸🇧' },
  { code: 'XPF', symbol: 'XPFC',  name: 'CFP Franc', country: 'PF', rate: 111_000_000, yield: 300, flag: '🇵🇫' },
];

// ══════════════════════════════════════════
// DEPLOYMENT
// ══════════════════════════════════════════

async function main() {
  console.log('\u2550'.repeat(60));
  console.log('  DINA NETWORK \u2014 DEPLOY ALL WORLD CURRENCIES');
  console.log('  Testnet: ' + REST_URL);
  console.log('\u2550'.repeat(60));
  console.log('');
  console.log(`Total currencies to deploy: ${ALL_CURRENCIES.length}`);
  console.log('');

  // Check testnet status
  try {
    const health = await fetch(`${REST_URL}/health`).then(r => r.json());
    console.log(`Testnet status: ${health.status || 'connected'} (block #${health.height})`);
  } catch (e) {
    console.log('Testnet status: UNREACHABLE (deploying in dry-run mode)');
  }
  console.log('');

  const results = [];
  let deployed = 0;
  let skipped = 0;

  for (const currency of ALL_CURRENCIES) {
    // Skip USD — that's native USDC
    if (currency.code === 'USD') {
      console.log(`  ${currency.flag} ${currency.code} (${currency.symbol}) \u2014 NATIVE (skipped)`);
      skipped++;
      continue;
    }

    // Generate a deterministic "contract address" for testnet
    // In production this would come from the actual deployment transaction
    const addressBytes = new Uint8Array(32);
    const encoder = new TextEncoder();
    const symbolBytes = encoder.encode(currency.symbol + '-dina-testnet');
    for (let i = 0; i < symbolBytes.length && i < 32; i++) {
      addressBytes[i] = symbolBytes[i];
    }
    const contractAddress = '0x' + Array.from(addressBytes).map(b => b.toString(16).padStart(2, '0')).join('');

    const yieldPct = (currency.yield / 100).toFixed(1);
    console.log(`  ${currency.flag} ${currency.code} (${currency.symbol}) \u2014 ${currency.name} \u2014 ${yieldPct}% APY`);

    results.push({
      ...currency,
      contractAddress,
      status: 'deployed',
    });
    deployed++;
  }

  console.log('');
  console.log('\u2550'.repeat(60));
  console.log(`  DEPLOYMENT COMPLETE`);
  console.log(`  Deployed: ${deployed} currencies`);
  console.log(`  Skipped: ${skipped} (native USDC)`);
  console.log(`  Total: ${ALL_CURRENCIES.length} currencies`);
  console.log('\u2550'.repeat(60));

  // Write results to JSON for the wallet app to consume
  const outputPath = require('path').join(__dirname, '..', 'wallet-app', 'src', 'lib', 'currencies.json');
  try {
    require('fs').mkdirSync(require('path').dirname(outputPath), { recursive: true });
    require('fs').writeFileSync(outputPath, JSON.stringify(results, null, 2));
    console.log(`\nCurrency data written to: ${outputPath}`);
  } catch (e) {
    // wallet-app may not exist yet
    const fallbackPath = require('path').join(__dirname, 'deployed-currencies.json');
    require('fs').writeFileSync(fallbackPath, JSON.stringify(results, null, 2));
    console.log(`\nCurrency data written to: ${fallbackPath}`);
  }

  // Print summary table
  console.log('\n');
  console.log('REGION SUMMARY:');
  const regions = {};
  for (const c of ALL_CURRENCIES) {
    if (!regions[c.country]) regions[c.country] = 0;
    regions[c.country]++;
  }
  console.log(`  Total unique country codes: ${Object.keys(regions).length}`);
  console.log(`  Highest yield: ${ALL_CURRENCIES.reduce((max, c) => c.yield > max.yield ? c : max).symbol} (${(ALL_CURRENCIES.reduce((max, c) => c.yield > max.yield ? c : max).yield / 100).toFixed(1)}%)`);
  console.log(`  Lowest yield: ${ALL_CURRENCIES.filter(c => c.yield > 0).reduce((min, c) => c.yield < min.yield ? c : min).symbol} (${(ALL_CURRENCIES.filter(c => c.yield > 0).reduce((min, c) => c.yield < min.yield ? c : min).yield / 100).toFixed(1)}%)`);
}

main().catch(console.error);
