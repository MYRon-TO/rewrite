package rewrite

// A generator is a declarative value-source contract.
#Generator: #LegacyGenerator | #MachineSecretGenerator | #CorpusChoiceGenerator

#GeneratorBase: {
	id:           #Identifier
	description?: string
}

#LegacyGenerator: close({
	#GeneratorBase
	type: #LegacyGeneratorType
	params: *{} | #MVPJSONObject
})

#MachineSecretGenerator: close({
	#GeneratorBase
	type:   "machine_secret"
	params: #MachineSecretParams
})

#CorpusChoiceGenerator: close({
	#GeneratorBase
	type:   "corpus_choice"
	params: #CorpusChoiceParams
})

#LegacyGeneratorType:  "fixed" | "const" | "choice" | "number_range" | "range" | "ipv4" | "username" | "simple_username" | "uuid" | "session_id" | "sequence" | "simple_words" | "simple_filename"
#BuiltinGeneratorType: #LegacyGeneratorType | "machine_secret" | "corpus_choice"

#Encoding: "base64url" | "hex" | "alphanumeric"
#Length:   int & >=1 & <=4096

#MachineSecretParams:
	close({profile: "hex", byte_length: #Length}) |
	close({profile: "base64", byte_length: #Length}) |
	close({profile: "base64url", byte_length: #Length}) |
	close({profile: "uuid"}) |
	close({profile: "api_key", prefix: *"sim_sk_" | string, body_length: *32 | #Length, encoding: *"base64url" | #Encoding}) |
	close({profile: "bearer_token", prefix: *"sim_bearer_" | string, body_length: *40 | #Length, encoding: *"base64url" | #Encoding}) |
	close({profile: "jwt_shape", issuer: *"simulator" | string, subject: *"simulated-user" | string, signature_length: *32 | #Length}) |
	close({profile: "pem_shape", label: *"SIMULATED PRIVATE KEY" | string, byte_length: *64 | #Length, line_width: *64 | (int & >=1 & <=256)}) |
	close({profile: "cloud_access_key", prefix: *"SIMAKIA" | string, body_length: *16 | #Length}) |
	close({profile: "cloud_secret_pair", id_prefix: *"SIMAKIA" | string, id_body_length: *16 | #Length, secret_length: *40 | #Length}) |
	close({profile: "connection_string", scheme: *"postgresql" | (string & !=""), host: *"db.invalid" | (string & !=""), port: *5432 | (int & >=1 & <=65535), database: *"simdb" | (string & !=""), username: *"sim_user" | (string & !=""), password_length: *32 | #Length}) |
	close({profile: "private_token", prefix: *"sim_tok_" | string, body_length: *32 | #Length, encoding: *"base64url" | #Encoding})

#CorpusSource: close({builtin: string}) | close({path: string})
#CorpusChoiceParams: close({
	source:      #CorpusSource
	min_length:  *1 | (int & >=0 & <=4096)
	max_length:  *(128 & >=min_length) | (int & >=0 & <=4096 & >=min_length)
	sampling:    *"uniform" | "uniform" | "shuffle_cycle"
	deduplicate: *true | bool
	keep_empty:  *false | bool
})
