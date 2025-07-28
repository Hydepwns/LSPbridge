/// Real-world code examples for testing semantic context extraction

pub const TYPESCRIPT_REACT_COMPONENT: &str = r#"
import React, { useState, useEffect, useCallback } from 'react';
import { User, ApiResponse } from '../types/api';
import { userService } from '../services/user.service';
import { Logger } from '../utils/logger';

interface UserListProps {
    initialUsers?: User[];
    onUserSelect?: (user: User) => void;
    className?: string;
}

interface UserListState {
    users: User[];
    loading: boolean;
    error: string | null;
    selectedUserId: number | null;
}

export const UserList: React.FC<UserListProps> = ({ 
    initialUsers = [], 
    onUserSelect,
    className = 'user-list'
}) => {
    const [state, setState] = useState<UserListState>({
        users: initialUsers,
        loading: false,
        error: null,
        selectedUserId: null,
    });

    const logger = new Logger('UserList');

    const fetchUsers = useCallback(async () => {
        setState(prev => ({ ...prev, loading: true, error: null }));
        
        try {
            const response: ApiResponse<User[]> = await userService.getAllUsers();
            
            if (response.success) {
                setState(prev => ({ 
                    ...prev, 
                    users: response.data,
                    loading: false 
                }));
            } else {
                throw new Error(response.error || 'Failed to fetch users');
            }
        } catch (error) {
            logger.error('Failed to fetch users', error);
            setState(prev => ({ 
                ...prev, 
                error: error.message,
                loading: false 
            }));
        }
    }, []);

    useEffect(() => {
        if (initialUsers.length === 0) {
            fetchUsers();
        }
    }, [initialUsers.length, fetchUsers]);

    const handleUserClick = useCallback((user: User) => {
        setState(prev => ({ ...prev, selectedUserId: user.id }));
        
        // Type error: onUserSelect might be undefined
        onUserSelect(user);
    }, [onUserSelect]);

    const handleRetry = useCallback(() => {
        fetchUsers();
    }, [fetchUsers]);

    if (state.loading) {
        return <div className="loading-spinner">Loading users...</div>;
    }

    if (state.error) {
        return (
            <div className="error-container">
                <p>Error: {state.error}</p>
                <button onClick={handleRetry}>Retry</button>
            </div>
        );
    }

    return (
        <div className={className}>
            <h2>Users ({state.users.length})</h2>
            <ul className="user-list__items">
                {state.users.map(user => (
                    <li 
                        key={user.id}
                        className={`user-item ${state.selectedUserId === user.id ? 'selected' : ''}`}
                        onClick={() => handleUserClick(user)}
                    >
                        <div className="user-info">
                            <h3>{user.name}</h3>
                            <p>{user.email}</p>
                            {user.department && <span className="department">{user.department}</span>}
                        </div>
                    </li>
                ))}
            </ul>
        </div>
    );
};

export default UserList;
"#;

pub const RUST_WEB_SERVER: &str = r#"
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use warp::{Filter, Rejection, Reply};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UserRepository {
    users: Arc<RwLock<HashMap<Uuid, User>>>,
}

impl UserRepository {
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User, String> {
        let mut users = self.users.write().await;
        
        // Check if email already exists
        for user in users.values() {
            if user.email == request.email {
                return Err("Email already exists".to_string());
            }
        }

        let user = User {
            id: Uuid::new_v4(),
            name: request.name,
            email: request.email,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        users.insert(user.id, user.clone());
        Ok(user)
    }

    pub async fn get_user(&self, id: Uuid) -> Option<User> {
        let users = self.users.read().await;
        users.get(&id).cloned()
    }

    pub async fn update_user(&self, id: Uuid, request: UpdateUserRequest) -> Result<User, String> {
        let mut users = self.users.write().await;
        
        let user = users.get_mut(&id).ok_or("User not found")?;

        if let Some(name) = request.name {
            user.name = name;
        }
        
        if let Some(email) = request.email {
            // Check if email already exists for another user
            for (other_id, other_user) in users.iter() {
                if *other_id != id && other_user.email == email {
                    return Err("Email already exists".to_string());
                }
            }
            user.email = email;
        }

        user.updated_at = Utc::now();
        Ok(user.clone())
    }

    pub async fn delete_user(&self, id: Uuid) -> Result<(), String> {
        let mut users = self.users.write().await;
        users.remove(&id).ok_or("User not found")?;
        Ok(())
    }

    pub async fn list_users(&self) -> Vec<User> {
        let users = self.users.read().await;
        users.values().cloned().collect()
    }
}

pub struct UserService {
    repository: UserRepository,
}

impl UserService {
    pub fn new(repository: UserRepository) -> Self {
        Self { repository }
    }

    pub async fn create_user(&self, request: CreateUserRequest) -> Result<User, String> {
        // Validation
        if request.name.trim().is_empty() {
            return Err("Name cannot be empty".to_string());
        }

        if !request.email.contains('@') {
            return Err("Invalid email format".to_string());
        }

        self.repository.create_user(request).await
    }

    pub async fn get_user_by_id(&self, id: Uuid) -> Result<User, String> {
        self.repository.get_user(id).await
            .ok_or_else(|| "User not found".to_string())
    }

    pub async fn update_user(&self, id: Uuid, request: UpdateUserRequest) -> Result<User, String> {
        // Validation
        if let Some(ref name) = request.name {
            if name.trim().is_empty() {
                return Err("Name cannot be empty".to_string());
            }
        }

        if let Some(ref email) = request.email {
            if !email.contains('@') {
                return Err("Invalid email format".to_string());
            }
        }

        self.repository.update_user(id, request).await
    }

    pub async fn delete_user(&self, id: Uuid) -> Result<(), String> {
        self.repository.delete_user(id).await
    }

    pub async fn list_all_users(&self) -> Vec<User> {
        self.repository.list_users().await
    }
}

// Error in this function: trying to move out of borrowed content
pub async fn setup_routes(service: UserService) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let service = Arc::new(service);
    
    let create_user = warp::path("users")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_service(service.clone()))
        .and_then(handle_create_user);

    let get_user = warp::path!("users" / Uuid)
        .and(warp::get())
        .and(with_service(service.clone()))
        .and_then(handle_get_user);

    // Type error: service moved in previous line
    let update_user = warp::path!("users" / Uuid)
        .and(warp::put())
        .and(warp::body::json())
        .and(with_service(service.clone()))
        .and_then(handle_update_user);

    create_user
        .or(get_user)
        .or(update_user)
}

fn with_service(service: Arc<UserService>) -> impl Filter<Extract = (Arc<UserService>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || service.clone())
}

async fn handle_create_user(request: CreateUserRequest, service: Arc<UserService>) -> Result<impl Reply, Rejection> {
    match service.create_user(request).await {
        Ok(user) => Ok(warp::reply::json(&user)),
        Err(error) => Err(warp::reject::custom(ApiError::BadRequest(error))),
    }
}

async fn handle_get_user(id: Uuid, service: Arc<UserService>) -> Result<impl Reply, Rejection> {
    match service.get_user_by_id(id).await {
        Ok(user) => Ok(warp::reply::json(&user)),
        Err(error) => Err(warp::reject::custom(ApiError::NotFound(error))),
    }
}

async fn handle_update_user(id: Uuid, request: UpdateUserRequest, service: Arc<UserService>) -> Result<impl Reply, Rejection> {
    match service.update_user(id, request).await {
        Ok(user) => Ok(warp::reply::json(&user)),
        Err(error) => Err(warp::reject::custom(ApiError::BadRequest(error))),
    }
}

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    NotFound(String),
    InternalServerError(String),
}

impl warp::reject::Reject for ApiError {}
"#;

pub const PYTHON_DATA_PROCESSING: &str = r#"
import pandas as pd
import numpy as np
from typing import List, Dict, Optional, Union, Tuple
from dataclasses import dataclass
from datetime import datetime, timedelta
import logging
from pathlib import Path
import json

logger = logging.getLogger(__name__)

@dataclass
class DataProcessingConfig:
    input_file: str
    output_file: str
    batch_size: int = 1000
    enable_validation: bool = True
    error_threshold: float = 0.05
    
class DataProcessor:
    """A class for processing large datasets with validation and error handling."""
    
    def __init__(self, config: DataProcessingConfig):
        self.config = config
        self.errors: List[Dict] = []
        self.processed_count = 0
        self.total_count = 0
        
    def load_data(self) -> pd.DataFrame:
        """Load data from the configured input file."""
        try:
            file_path = Path(self.config.input_file)
            
            if not file_path.exists():
                raise FileNotFoundError(f"Input file not found: {self.config.input_file}")
            
            # Support multiple file formats
            if file_path.suffix.lower() == '.csv':
                df = pd.read_csv(file_path)
            elif file_path.suffix.lower() in ['.xlsx', '.xls']:
                df = pd.read_excel(file_path)
            elif file_path.suffix.lower() == '.json':
                df = pd.read_json(file_path)
            else:
                raise ValueError(f"Unsupported file format: {file_path.suffix}")
            
            logger.info(f"Loaded {len(df)} records from {self.config.input_file}")
            self.total_count = len(df)
            return df
            
        except Exception as e:
            logger.error(f"Failed to load data: {str(e)}")
            raise
    
    def validate_row(self, row: pd.Series, row_index: int) -> bool:
        """Validate a single row of data."""
        errors = []
        
        # Check for required fields
        required_fields = ['id', 'name', 'email', 'created_at']
        for field in required_fields:
            if field not in row or pd.isna(row[field]):
                errors.append(f"Missing required field: {field}")
        
        # Validate email format
        if 'email' in row and not pd.isna(row['email']):
            email = str(row['email'])
            if '@' not in email or '.' not in email:
                errors.append(f"Invalid email format: {email}")
        
        # Validate date format
        if 'created_at' in row and not pd.isna(row['created_at']):
            try:
                if isinstance(row['created_at'], str):
                    datetime.fromisoformat(row['created_at'])
            except ValueError:
                errors.append(f"Invalid date format: {row['created_at']}")
        
        # Validate numeric fields
        if 'age' in row and not pd.isna(row['age']):
            try:
                age = float(row['age'])
                if age < 0 or age > 150:
                    errors.append(f"Invalid age: {age}")
            except (ValueError, TypeError):
                errors.append(f"Age must be numeric: {row['age']}")
        
        if errors:
            self.errors.append({
                'row_index': row_index,
                'errors': errors,
                'data': row.to_dict()
            })
            return False
        
        return True
    
    def transform_row(self, row: pd.Series) -> Dict:
        """Transform a single row of data."""
        transformed = {}
        
        # Copy basic fields
        for field in ['id', 'name', 'email']:
            if field in row and not pd.isna(row[field]):
                transformed[field] = row[field]
        
        # Transform date fields
        if 'created_at' in row and not pd.isna(row['created_at']):
            if isinstance(row['created_at'], str):
                transformed['created_at'] = datetime.fromisoformat(row['created_at'])
            else:
                transformed['created_at'] = row['created_at']
        
        # Calculate derived fields
        if 'age' in row and not pd.isna(row['age']):
            age = float(row['age'])
            transformed['age'] = age
            transformed['age_group'] = self.get_age_group(age)
        
        # Add processing metadata
        transformed['processed_at'] = datetime.now()
        transformed['processor_version'] = '1.0.0'
        
        return transformed
    
    def get_age_group(self, age: float) -> str:
        """Categorize age into groups."""
        if age < 18:
            return 'minor'
        elif age < 30:
            return 'young_adult'
        elif age < 50:
            return 'adult'
        elif age < 65:
            return 'middle_aged'
        else:
            return 'senior'
    
    def process_batch(self, batch: pd.DataFrame, batch_number: int) -> List[Dict]:
        """Process a batch of data."""
        results = []
        
        logger.info(f"Processing batch {batch_number} with {len(batch)} records")
        
        for index, row in batch.iterrows():
            try:
                # Validation step
                if self.config.enable_validation:
                    if not self.validate_row(row, index):
                        continue
                
                # Transformation step
                transformed = self.transform_row(row)
                results.append(transformed)
                self.processed_count += 1
                
            except Exception as e:
                logger.error(f"Error processing row {index}: {str(e)}")
                self.errors.append({
                    'row_index': index,
                    'errors': [f"Processing error: {str(e)}"],
                    'data': row.to_dict() if hasattr(row, 'to_dict') else str(row)
                })
        
        return results
    
    def check_error_threshold(self) -> bool:
        """Check if error rate exceeds threshold."""
        if self.total_count == 0:
            return True
        
        error_rate = len(self.errors) / self.total_count
        if error_rate > self.config.error_threshold:
            logger.warning(f"Error rate {error_rate:.2%} exceeds threshold {self.config.error_threshold:.2%}")
            return False
        
        return True
    
    def save_results(self, results: List[Dict]) -> None:
        """Save processed results to output file."""
        if not results:
            logger.warning("No results to save")
            return
        
        output_path = Path(self.config.output_file)
        output_path.parent.mkdir(parents=True, exist_ok=True)
        
        # Convert to DataFrame for easier saving
        df = pd.DataFrame(results)
        
        # Save based on file extension
        if output_path.suffix.lower() == '.csv':
            df.to_csv(output_path, index=False)
        elif output_path.suffix.lower() in ['.xlsx', '.xls']:
            df.to_excel(output_path, index=False)
        elif output_path.suffix.lower() == '.json':
            df.to_json(output_path, orient='records', date_format='iso')
        else:
            # Default to JSON
            df.to_json(output_path, orient='records', date_format='iso')
        
        logger.info(f"Saved {len(results)} processed records to {self.config.output_file}")
    
    def save_error_report(self) -> None:
        """Save error report to file."""
        if not self.errors:
            return
        
        error_file = Path(self.config.output_file).with_suffix('.errors.json')
        
        error_report = {
            'total_errors': len(self.errors),
            'error_rate': len(self.errors) / self.total_count if self.total_count > 0 else 0,
            'errors': self.errors
        }
        
        with open(error_file, 'w') as f:
            json.dump(error_report, f, indent=2, default=str)
        
        logger.info(f"Saved error report to {error_file}")
    
    def process(self) -> Dict:
        """Main processing method."""
        start_time = datetime.now()
        logger.info("Starting data processing")
        
        try:
            # Load data
            df = self.load_data()
            
            # Process in batches
            all_results = []
            batch_number = 0
            
            for i in range(0, len(df), self.config.batch_size):
                batch = df.iloc[i:i + self.config.batch_size]
                batch_number += 1
                
                # Process batch
                batch_results = self.process_batch(batch, batch_number)
                all_results.extend(batch_results)
                
                # Check error threshold periodically
                if batch_number % 10 == 0:
                    if not self.check_error_threshold():
                        raise ValueError("Error threshold exceeded, stopping processing")
            
            # Final error check
            if not self.check_error_threshold():
                logger.warning("Processing completed but error threshold was exceeded")
            
            # Save results
            self.save_results(all_results)
            self.save_error_report()
            
            end_time = datetime.now()
            processing_time = end_time - start_time
            
            summary = {
                'total_records': self.total_count,
                'processed_records': self.processed_count,
                'error_count': len(self.errors),
                'error_rate': len(self.errors) / self.total_count if self.total_count > 0 else 0,
                'processing_time': str(processing_time),
                'success': True
            }
            
            logger.info(f"Processing completed: {summary}")
            return summary
            
        except Exception as e:
            logger.error(f"Processing failed: {str(e)}")
            # Error: trying to return different types in same function
            return False

def main():
    """Example usage of the DataProcessor."""
    config = DataProcessingConfig(
        input_file='data/input.csv',
        output_file='data/output.json',
        batch_size=500,
        enable_validation=True,
        error_threshold=0.1
    )
    
    processor = DataProcessor(config)
    result = processor.process()
    
    # Type error: result could be Dict or bool
    print(f"Processing completed with {result['processed_records']} records")

if __name__ == '__main__':
    main()
"#;