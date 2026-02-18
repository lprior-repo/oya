{-# LANGUAGE OverloadedStrings #-}
{-# LANGUAGE DeriveGeneric #-}

module Main where

import Web.Scotty
import Data.Aeson (FromJSON, ToJSON)
import qualified Data.Text as T
import qualified Data.Map as M
import Data.IORef (IORef, newIORef, readIORef, modifyIORef)
import Control.Monad.IO.Class (liftIO)
import Network.HTTP.Types.Status (Status, status201, status204, status400, status404, status409)
import GHC.Generics (Generic)
import Data.Char (isAlphaNum, isAscii, isControl, isSpace)

-- Data types
data User = User
    { userId :: Int
    , userName :: T.Text
    , userEmail :: T.Text
    } deriving (Show, Generic)

instance ToJSON User
instance FromJSON User

data CreateUserRequest = CreateUserRequest
    { name :: T.Text
    , email :: T.Text
    } deriving (Show, Generic)

instance FromJSON CreateUserRequest

-- Type alias for our in-memory store
type UserStore = IORef (M.Map Int User)

jsonError :: Status -> T.Text -> ActionM ()
jsonError errorStatus message = do
    status errorStatus
    json $ M.fromList [("error" :: T.Text, message)]

parseUserId :: ActionM (Either T.Text Int)
parseUserId = do
    result <- (Right <$> param "id") `rescue` (const $ pure $ Left "Invalid user id")
    pure $ case result of
        Left errorMessage -> Left errorMessage
        Right uid | uid <= 0 -> Left "Invalid user id"
        Right uid -> Right uid

parseCreateUserRequest :: ActionM (Either T.Text CreateUserRequest)
parseCreateUserRequest = do
    result <-
        (Right <$> jsonData)
            `rescue` (const $ pure $ Left "Invalid request body")
    pure $ case result of
        Left errorMessage -> Left errorMessage
        Right requestPayload -> validateCreateUserRequest requestPayload

maxNameLength :: Int
maxNameLength = 100

maxEmailLength :: Int
maxEmailLength = 254

validateCreateUserRequest :: CreateUserRequest -> Either T.Text CreateUserRequest
validateCreateUserRequest requestPayload = do
    validName <- validateName (name requestPayload)
    validEmail <- validateEmail (email requestPayload)
    pure $ CreateUserRequest validName validEmail

validateName :: T.Text -> Either T.Text T.Text
validateName nameValue = validateTextField "name" maxNameLength nameValue

validateEmail :: T.Text -> Either T.Text T.Text
validateEmail emailValue = do
    normalizedEmail <- validateTextField "email" maxEmailLength emailValue
    if T.any isSpace normalizedEmail
        then Left "email contains whitespace"
        else
            if isValidEmail normalizedEmail
                then Right normalizedEmail
                else Left "email format is invalid"

validateTextField :: T.Text -> Int -> T.Text -> Either T.Text T.Text
validateTextField fieldName maxLength fieldValue =
    let normalizedValue = T.strip fieldValue
     in if T.null normalizedValue
            then Left (fieldName <> " cannot be empty")
            else
                if T.length normalizedValue > maxLength
                    then Left (fieldName <> " is too long")
                    else
                        if T.any isControl normalizedValue
                            then Left (fieldName <> " contains control characters")
                            else Right normalizedValue

isValidEmail :: T.Text -> Bool
isValidEmail emailValue =
    let splitEmail = T.splitOn "@" emailValue
        allowedChar c = isAscii c && (isAlphaNum c || c `elem` ("._%+-@-" :: String))
     in case splitEmail of
            [localPart, domainPart] ->
                not (T.null localPart)
                    && not (T.null domainPart)
                    && T.length localPart <= 64
                    && T.any (== '.') domainPart
                    && not (T.isPrefixOf "." domainPart)
                    && not (T.isSuffixOf "." domainPart)
                    && T.all allowedChar emailValue
            _ -> False

emailExistsInStore :: Int -> T.Text -> M.Map Int User -> Bool
emailExistsInStore ignoredUserId emailValue users =
    let normalizedEmail = T.toCaseFold emailValue
        matchesEmail user =
            userId user /= ignoredUserId
                && T.toCaseFold (userEmail user) == normalizedEmail
     in any matchesEmail (M.elems users)

-- Main entry point
main :: IO ()
main = do
    putStrLn "Starting Haskell API server on port 3000..."
    store <- newIORef M.empty
    scotty 3000 (app store)

-- Application routes
app :: UserStore -> ScottyM ()
app store = do
    -- Health check
    get "/health" $ do
        json $ M.fromList [("status" :: T.Text, "healthy" :: T.Text)]
    
    -- Get all users
    get "/api/users" $ do
        users <- liftIO $ M.elems <$> readIORef store
        json users
    
    -- Get user by ID
    get "/api/users/:id" $ do
        parsedId <- parseUserId
        case parsedId of
            Left errorMessage -> jsonError status400 errorMessage
            Right uid -> do
                users <- liftIO $ readIORef store
                case M.lookup uid users of
                    Just user -> json user
                    Nothing -> jsonError status404 "User not found"
    
    -- Create user
    post "/api/users" $ do
        parsedRequest <- parseCreateUserRequest
        case parsedRequest of
            Left errorMessage -> jsonError status400 errorMessage
            Right req -> do
                users <- liftIO $ readIORef store
                if emailExistsInStore 0 (email req) users
                    then jsonError status409 "email already exists"
                    else do
                        let newId = if M.null users then 1 else fst (M.findMax users) + 1
                        let newUser = User newId (name req) (email req)
                        liftIO $ modifyIORef store (M.insert newId newUser)
                        status status201
                        json newUser
    
    -- Update user
    put "/api/users/:id" $ do
        parsedId <- parseUserId
        parsedRequest <- parseCreateUserRequest
        case (parsedId, parsedRequest) of
            (Left errorMessage, _) -> jsonError status400 errorMessage
            (_, Left errorMessage) -> jsonError status400 errorMessage
            (Right uid, Right req) -> do
                users <- liftIO $ readIORef store
                case M.lookup uid users of
                    Just _ -> do
                        if emailExistsInStore uid (email req) users
                            then jsonError status409 "email already exists"
                            else do
                                let updatedUser = User uid (name req) (email req)
                                liftIO $ modifyIORef store (M.insert uid updatedUser)
                                json updatedUser
                    Nothing -> jsonError status404 "User not found"
    
    -- Delete user
    delete "/api/users/:id" $ do
        parsedId <- parseUserId
        case parsedId of
            Left errorMessage -> jsonError status400 errorMessage
            Right uid -> do
                users <- liftIO $ readIORef store
                case M.lookup uid users of
                    Just _ -> do
                        liftIO $ modifyIORef store (M.delete uid)
                        status status204
                    Nothing -> jsonError status404 "User not found"
    
    -- API info
    get "/" $ do
        json $ M.fromList 
            [ ("name" :: T.Text, "Haskell REST API" :: T.Text)
            , ("version", "0.1.0")
            , ("endpoints", [ "/health", "/api/users", "/api/users/:id" ] :: [T.Text])
            ]
