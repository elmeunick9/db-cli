INSERT INTO "market" VALUES
    ('ANY', 'Other assets',                     'USD', '14:30', '21:00'),
    ('NDQ', 'NASDAQ',                           'USD', '14:30', '21:00'),
    ('NSY', 'NYSE: New York Stock Exchange',    'USD', '14:30', '21:00');

INSERT INTO "provider" VALUES
    ('YAHOO'    , 'Yahoo! Finance'),
    ('DEGIRO'   , 'Degiro'),
    ('TV-S'     , 'TradingView Screener'),
    ('TV-C'     , 'TradingView Chart'),
    ('ALPHA'    , 'Alpha Vantage');

/* General Market Index & Other Indicators */
INSERT INTO "listing" VALUES
    ('IDX', '^GSPC', 'S&P 500',                        'ANY', true),
    ('IDX', '^DJI' , 'Dow 30',                         'ANY', true),
    ('IDX', '^IXIC', 'Nasdaq',                         'ANY', true),
    ('IDX', '^RUT' , 'Russell 2000',                   'ANY', true),
    ('IDX', 'CL=F' , 'Crude Oil',                      'ANY', true),
    ('IDX', 'GC=F' , 'Gold',                           'ANY', true),
    ('IDX', 'SI=F' , 'Silver',                         'ANY', true),
    ('IDX', '^TNX' , '10-Yr USA Bond',                 'ANY', true),
    ('FX' , 'EURUSD=X', 'EUR/USD',                     'ANY', true);
