{-# LANGUAGE DeriveGeneric #-}

module Pokemon.Types where

import Data.Text (Text)
import Data.Aeson (FromJSON, ToJSON, encode, decode)

data PokemonType = Fire | Water | Grass | Electric
  deriving (Show, Eq)

data PokemonStats = PokemonStats
  { hp :: Int
  , attack :: Int
  , defense :: Int
  } deriving (Show, Eq)

data Pokemon = Pokemon
  { name :: Text
  , ptype :: PokemonType
  , stats :: PokemonStats
  } deriving (Show, Eq, ToJSON, FromJSON)

pikachu :: Pokemon
pikachu = Pokemon
  { name = "Pikachu"
  , ptype = Electric
  , stats = PokemonStats { hp = 35, attack = 55, defense = 40 }
  }

charmander :: Pokemon
charmander = Pokemon
  { name = "Charmander"
  , ptype = Fire
  , stats = PokemonStats { hp = 39, attack = 52, defense = 43 }
  }
