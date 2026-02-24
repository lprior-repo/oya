module Main where

import Pokemon.Types
import qualified Data.Aeson as Aeson
import qualified Data.ByteString.Lazy as BSL

main :: IO ()
main = do
    putStrLn "=== Pokemon API Demo ==="
    
    -- Test encoding
    let json = Aeson.encode pikachu
    BSL.putStrLn json
    putStrLn ""
    
    -- Test decoding
    case Aeson.eitherDecode json :: Either String Pokemon of
        Left err -> putStrLn $ "Decode error: " ++ err
        Right p -> do
            putStrLn "Decoded Pokemon:"
            putStrLn $ "  Name: " ++ show (name p)
            putStrLn $ "  Type: " ++ show (ptype p)
            putStrLn $ "  HP: " ++ show (hp (stats p))
