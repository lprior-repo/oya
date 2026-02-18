{-# LANGUAGE OverloadedStrings #-}

module Spec where

import Test.Hspec
import Test.Hspec.Wai
import Test.Hspec.Wai.JSON
import Web.Scotty (scottyApp)
import Data.IORef (newIORef)
import qualified Data.Map as M
import Main (app)
import Network.HTTP.Types.Header (hContentType)
import Network.Wai (Application)
import qualified Data.ByteString.Lazy.Char8 as L8

main :: IO ()
main = hspec spec

spec :: Spec
spec = with (createApp) $ do
    describe "GET /health" $ do
        it "returns healthy status" $ do
            get "/health" `shouldRespondWith` 
                [json|{status: "healthy"}|]
                { matchStatus = 200
                , matchHeaders = [hContentType <:> "application/json; charset=utf-8"]
                }

    describe "GET /" $ do
        it "returns API info" $ do
            get "/" `shouldRespondWith` 200

    describe "User API" $ do
        describe "POST /api/users" $ do
            it "creates user with correct data" $ do
                let user = [json|{"name": "Bob", "email": "bob@example.com"}|]
                post "/api/users" user `shouldRespondWith` 
                    [json|{"userId": 1, "userName": "Bob", "userEmail": "bob@example.com"}|]
                    { matchStatus = 201 }

            it "assigns incrementing ids" $ do
                let firstUser = [json|{"name": "Alice", "email": "alice@example.com"}|]
                let secondUser = [json|{"name": "Carol", "email": "carol@example.com"}|]
                post "/api/users" firstUser `shouldRespondWith` 201
                post "/api/users" secondUser `shouldRespondWith`
                    [json|{"userId": 2, "userName": "Carol", "userEmail": "carol@example.com"}|]
                    { matchStatus = 201 }

            it "returns 400 for invalid request body" $ do
                post "/api/users" "not-json" `shouldRespondWith`
                    [json|{"error": "Invalid request body"}|]
                    { matchStatus = 400 }

            it "returns 400 for missing required fields" $ do
                post "/api/users" [json|{"name": "OnlyName"}|] `shouldRespondWith`
                    [json|{"error": "Invalid request body"}|]
                    { matchStatus = 400 }

            it "rejects empty name" $ do
                post "/api/users" [json|{"name": "   ", "email": "emptyname@example.com"}|] `shouldRespondWith`
                    [json|{"error": "name cannot be empty"}|]
                    { matchStatus = 400 }

            it "rejects invalid email format" $ do
                post "/api/users" [json|{"name": "NoAt", "email": "no-at-symbol"}|] `shouldRespondWith`
                    [json|{"error": "email format is invalid"}|]
                    { matchStatus = 400 }

            it "rejects email with whitespace" $ do
                post "/api/users" [json|{"name": "Space", "email": "bad @example.com"}|] `shouldRespondWith`
                    [json|{"error": "email contains whitespace"}|]
                    { matchStatus = 400 }

            it "rejects duplicate email" $ do
                let firstUser = [json|{"name": "Primary", "email": "dup@example.com"}|]
                let secondUser = [json|{"name": "Secondary", "email": "DUP@example.com"}|]
                post "/api/users" firstUser `shouldRespondWith` 201
                post "/api/users" secondUser `shouldRespondWith`
                    [json|{"error": "email already exists"}|]
                    { matchStatus = 409 }

            it "accepts boundary-length name and email" $ do
                let maxName = replicate 100 'n'
                let local = replicate 64 'a'
                let domainLabel = replicate 63 'b'
                let maxEmail = local <> "@" <> domainLabel <> "." <> domainLabel <> "." <> "co"
                let body = L8.pack $ "{\"name\":\"" <> maxName <> "\",\"email\":\"" <> maxEmail <> "\"}"
                post "/api/users" body `shouldRespondWith` 201

            it "rejects name longer than boundary" $ do
                let tooLongName = replicate 101 'x'
                let body = L8.pack $ "{\"name\":\"" <> tooLongName <> "\",\"email\":\"toolong@example.com\"}"
                post "/api/users" body `shouldRespondWith`
                    [json|{"error": "name is too long"}|]
                    { matchStatus = 400 }

            it "rejects email longer than boundary" $ do
                let local = replicate 64 'a'
                let labelA = replicate 63 'b'
                let labelB = replicate 63 'c'
                let labelC = replicate 63 'd'
                let tooLongEmail = local <> "@" <> labelA <> "." <> labelB <> "." <> labelC
                let body = L8.pack $ "{\"name\":\"Long Email\",\"email\":\"" <> tooLongEmail <> "\"}"
                post "/api/users" body `shouldRespondWith`
                    [json|{"error": "email is too long"}|]
                    { matchStatus = 400 }

            it "rejects control characters" $ do
                post "/api/users" [json|{"name": "Bad\nName", "email": "bad@example.com"}|] `shouldRespondWith`
                    [json|{"error": "name contains control characters"}|]
                    { matchStatus = 400 }

            it "rejects malformed JSON" $ do
                post "/api/users" "{\"name\":\"Broken\",\"email\":" `shouldRespondWith`
                    [json|{"error": "Invalid request body"}|]
                    { matchStatus = 400 }

            it "rejects wrong field types" $ do
                post "/api/users" [json|{"name": 123, "email": true}|] `shouldRespondWith`
                    [json|{"error": "Invalid request body"}|]
                    { matchStatus = 400 }

        describe "GET /api/users" $ do
            it "returns empty list when there are no users" $ do
                get "/api/users" `shouldRespondWith`
                    [json|[]|]
                    { matchStatus = 200 }

            it "returns list of created users" $ do
                let user = [json|{"name": "List", "email": "list@example.com"}|]
                post "/api/users" user `shouldRespondWith` 201
                get "/api/users" `shouldRespondWith`
                    [json|[{"userId": 1, "userName": "List", "userEmail": "list@example.com"}]|]
                    { matchStatus = 200 }

        describe "GET /api/users/:id" $ do
            it "returns user when found" $ do
                let user = [json|{"name": "Charlie", "email": "charlie@example.com"}|]
                post "/api/users" user
                get "/api/users/1" `shouldRespondWith`
                    [json|{"userId": 1, "userName": "Charlie", "userEmail": "charlie@example.com"}|]
                    { matchStatus = 200 }
                
            it "returns 404 when user not found" $ do
                get "/api/users/999" `shouldRespondWith`
                    [json|{"error": "User not found"}|]
                    { matchStatus = 404 }

            it "returns 400 for invalid user id" $ do
                get "/api/users/not-a-number" `shouldRespondWith`
                    [json|{"error": "Invalid user id"}|]
                    { matchStatus = 400 }

            it "returns 400 for zero or negative user id" $ do
                get "/api/users/0" `shouldRespondWith`
                    [json|{"error": "Invalid user id"}|]
                    { matchStatus = 400 }
                get "/api/users/-1" `shouldRespondWith`
                    [json|{"error": "Invalid user id"}|]
                    { matchStatus = 400 }

        describe "PUT /api/users/:id" $ do
            it "updates existing user" $ do
                let createdUser = [json|{"name": "Initial", "email": "initial@example.com"}|]
                let updatedUser = [json|{"name": "Updated", "email": "updated@example.com"}|]
                post "/api/users" createdUser `shouldRespondWith` 201
                put "/api/users/1" updatedUser `shouldRespondWith`
                    [json|{"userId": 1, "userName": "Updated", "userEmail": "updated@example.com"}|]
                    { matchStatus = 200 }

            it "returns 404 when updating non-existent user" $ do
                let updatedUser = [json|{"name": "Ghost", "email": "ghost@example.com"}|]
                put "/api/users/999" updatedUser `shouldRespondWith`
                    [json|{"error": "User not found"}|]
                    { matchStatus = 404 }

            it "returns 400 for invalid update body" $ do
                put "/api/users/1" "not-json" `shouldRespondWith`
                    [json|{"error": "Invalid request body"}|]
                    { matchStatus = 400 }

            it "returns 400 for invalid user id on update" $ do
                let updatedUser = [json|{"name": "Any", "email": "any@example.com"}|]
                put "/api/users/not-a-number" updatedUser `shouldRespondWith`
                    [json|{"error": "Invalid user id"}|]
                    { matchStatus = 400 }

            it "returns 400 for zero or negative user id on update" $ do
                let updatedUser = [json|{"name": "Any", "email": "any@example.com"}|]
                put "/api/users/0" updatedUser `shouldRespondWith`
                    [json|{"error": "Invalid user id"}|]
                    { matchStatus = 400 }
                put "/api/users/-1" updatedUser `shouldRespondWith`
                    [json|{"error": "Invalid user id"}|]
                    { matchStatus = 400 }

            it "rejects duplicate email on update" $ do
                post "/api/users" [json|{"name": "One", "email": "one@example.com"}|] `shouldRespondWith` 201
                post "/api/users" [json|{"name": "Two", "email": "two@example.com"}|] `shouldRespondWith` 201
                put "/api/users/2" [json|{"name": "Two", "email": "ONE@example.com"}|] `shouldRespondWith`
                    [json|{"error": "email already exists"}|]
                    { matchStatus = 409 }

        describe "DELETE /api/users/:id" $ do
            it "deletes existing user" $ do
                let user = [json|{"name": "Delete", "email": "delete@example.com"}|]
                post "/api/users" user
                delete "/api/users/1" `shouldRespondWith` 204

                get "/api/users/1" `shouldRespondWith`
                    [json|{"error": "User not found"}|]
                    { matchStatus = 404 }
                
            it "returns 404 for non-existent user" $ do
                delete "/api/users/999" `shouldRespondWith`
                    [json|{"error": "User not found"}|]
                    { matchStatus = 404 }

            it "returns 400 for invalid user id on delete" $ do
                delete "/api/users/not-a-number" `shouldRespondWith`
                    [json|{"error": "Invalid user id"}|]
                    { matchStatus = 400 }

            it "returns 400 for zero or negative user id on delete" $ do
                delete "/api/users/0" `shouldRespondWith`
                    [json|{"error": "Invalid user id"}|]
                    { matchStatus = 400 }
                delete "/api/users/-1" `shouldRespondWith`
                    [json|{"error": "Invalid user id"}|]
                    { matchStatus = 400 }

createApp :: IO Application
createApp = do
    store <- newIORef M.empty
    scottyApp (app store)
